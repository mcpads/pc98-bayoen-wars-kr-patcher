use anyhow::{Context, Result, ensure};

use crate::pc98_graphics::{DIGITAL_RGBI_COLOR_COUNT, digital_rgbi_palette};

pub(crate) const TITLE_SCREEN_WIDTH: usize = 640;
pub(crate) const TITLE_SCREEN_HEIGHT: usize = 400;
pub(crate) const TITLE_DECODED_SIZE: usize = 0xf200;

const SCREEN_ROW_BYTES: usize = TITLE_SCREEN_WIDTH / 8;
const SCREEN_PLANE_BYTES: usize = SCREEN_ROW_BYTES * TITLE_SCREEN_HEIGHT;
const BACKGROUND_SOURCE_OFFSET: usize = 0xef00;
const BACKGROUND_WIDTH_BYTES: usize = 2;
const BACKGROUND_HEIGHT: usize = 96;
const ANIMATION_DESTINATION_OFFSET: usize = 0x4b24;
const ANIMATION_WIDTH_BYTES: usize = 8;
const ANIMATION_HEIGHT: usize = 48;

pub(crate) const TITLE_CONTENT_TRANSFERS: [TitleTransfer; 4] = [
    TitleTransfer::new(0x0000, 0x1408, 24, 224),
    TitleTransfer::new(0x5400, 0x0020, 16, 320),
    TitleTransfer::new(0xa400, 0x1930, 8, 240),
    TitleTransfer::new(0xc200, 0x1e38, 16, 160),
];

const TITLE_COPYRIGHT_TRANSFER: TitleTransfer = TitleTransfer::new(0xea00, 0x731e, 20, 16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TitleTransfer {
    pub(crate) source_offset: usize,
    pub(crate) destination_offset: usize,
    pub(crate) width_bytes: usize,
    pub(crate) height: usize,
}

impl TitleTransfer {
    pub(crate) const fn new(
        source_offset: usize,
        destination_offset: usize,
        width_bytes: usize,
        height: usize,
    ) -> Self {
        Self {
            source_offset,
            destination_offset,
            width_bytes,
            height,
        }
    }

    pub(crate) fn x(self) -> usize {
        self.destination_offset % SCREEN_ROW_BYTES * 8
    }

    pub(crate) fn y(self) -> usize {
        self.destination_offset / SCREEN_ROW_BYTES
    }

    pub(crate) fn contains(self, x: usize, y: usize) -> bool {
        (self.x()..self.x() + self.width_bytes * 8).contains(&x)
            && (self.y()..self.y() + self.height).contains(&y)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct TitleScreen {
    pub(crate) pixels: Vec<u8>,
}

pub(crate) fn render_title_screen(decoded: &[u8]) -> Result<TitleScreen> {
    render_title_screen_with_palette(decoded, &digital_rgbi_palette())
}

pub(crate) fn render_title_screen_with_palette(
    decoded: &[u8],
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> Result<TitleScreen> {
    let planes = render_title_planes(decoded)?;
    Ok(render_title_planes_with_palette(&planes, palette))
}

pub(crate) fn render_title_screen_with_animation(
    decoded: &[u8],
    animation_decoded: &[u8],
    animation_source_offset: usize,
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> Result<TitleScreen> {
    let mut planes = render_title_planes(decoded)?;
    copy_compact_brgi(
        animation_decoded,
        &mut planes,
        animation_source_offset,
        ANIMATION_DESTINATION_OFFSET,
        ANIMATION_WIDTH_BYTES,
        ANIMATION_HEIGHT,
    )?;
    Ok(render_title_planes_with_palette(&planes, palette))
}

pub(crate) fn render_title_color_indices_with_animation(
    decoded: &[u8],
    animation_decoded: &[u8],
    animation_source_offset: usize,
) -> Result<Vec<u8>> {
    let mut planes = render_title_planes(decoded)?;
    copy_compact_brgi(
        animation_decoded,
        &mut planes,
        animation_source_offset,
        ANIMATION_DESTINATION_OFFSET,
        ANIMATION_WIDTH_BYTES,
        ANIMATION_HEIGHT,
    )?;
    Ok(render_title_plane_indices(&planes))
}

fn render_title_planes(decoded: &[u8]) -> Result<[Vec<u8>; 4]> {
    require_decoded_title(decoded)?;
    let mut planes: [Vec<u8>; 4] = std::array::from_fn(|_| vec![0; SCREEN_PLANE_BYTES]);

    let mut destination = 0usize;
    for _ in 0..4 {
        for _ in 0..40 {
            copy_compact_brgi(
                decoded,
                &mut planes,
                BACKGROUND_SOURCE_OFFSET,
                destination,
                BACKGROUND_WIDTH_BYTES,
                BACKGROUND_HEIGHT,
            )?;
            destination += 2;
        }
        destination += 0x1dfe;
    }
    for transfer in TITLE_CONTENT_TRANSFERS
        .into_iter()
        .chain([TITLE_COPYRIGHT_TRANSFER])
    {
        copy_transfer(decoded, &mut planes, transfer)?;
    }
    Ok(planes)
}

fn render_title_planes_with_palette(
    planes: &[Vec<u8>; 4],
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> TitleScreen {
    let indices = render_title_plane_indices(planes);
    let mut pixels = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 3];
    for (pixel, color_index) in pixels.as_chunks_mut::<3>().0.iter_mut().zip(indices) {
        pixel.copy_from_slice(&palette[usize::from(color_index)]);
    }
    TitleScreen { pixels }
}

fn render_title_plane_indices(planes: &[Vec<u8>; 4]) -> Vec<u8> {
    let mut indices = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    for y in 0..TITLE_SCREEN_HEIGHT {
        for x in 0..TITLE_SCREEN_WIDTH {
            let mask = 0x80 >> (x % 8);
            let byte = y * SCREEN_ROW_BYTES + x / 8;
            indices[y * TITLE_SCREEN_WIDTH + x] = (0..4).fold(0u8, |color, plane| {
                color | (u8::from(planes[plane][byte] & mask != 0) << plane)
            });
        }
    }
    indices
}

pub(crate) fn replace_title_artwork_preserving_source_background(
    decoded: &mut [u8],
    screen_color_indices: &[u8],
    background_color_indices: &[u8],
) -> Result<()> {
    require_decoded_title(decoded)?;
    require_title_screen_indices(screen_color_indices)?;
    ensure!(
        !background_color_indices.is_empty()
            && background_color_indices
                .iter()
                .all(|index| usize::from(*index) < DIGITAL_RGBI_COLOR_COUNT),
        "title background color indices must be a nonempty subset of the 16-color palette"
    );
    require_visible_pixels_fit_content_transfers(screen_color_indices, background_color_indices)?;
    replace_title_content(decoded, screen_color_indices)
}

pub(crate) fn replace_title_content(decoded: &mut [u8], screen_color_indices: &[u8]) -> Result<()> {
    require_decoded_title(decoded)?;
    require_title_screen_indices(screen_color_indices)?;

    for transfer in TITLE_CONTENT_TRANSFERS {
        let plane_size = transfer.width_bytes * transfer.height;
        for plane in 0..4 {
            for local_y in 0..transfer.height {
                for byte_x in 0..transfer.width_bytes {
                    let mut value = 0u8;
                    for bit in 0..8 {
                        let x = transfer.x() + byte_x * 8 + bit;
                        let y = transfer.y() + local_y;
                        let index = screen_color_indices[y * TITLE_SCREEN_WIDTH + x];
                        if index & (1 << plane) != 0 {
                            value |= 0x80 >> bit;
                        }
                    }
                    let offset = transfer.source_offset
                        + plane * plane_size
                        + local_y * transfer.width_bytes
                        + byte_x;
                    *decoded
                        .get_mut(offset)
                        .context("title content transfer lies outside TITLE.DAT")? = value;
                }
            }
        }
    }
    Ok(())
}

fn require_decoded_title(decoded: &[u8]) -> Result<()> {
    ensure!(
        decoded.len() == TITLE_DECODED_SIZE,
        "TITLE.DAT decoded size changed"
    );
    Ok(())
}

fn require_title_screen_indices(screen_color_indices: &[u8]) -> Result<()> {
    ensure!(
        screen_color_indices.len() == TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT,
        "title artwork must contain exactly one color index per 640x400 screen pixel"
    );
    ensure!(
        screen_color_indices
            .iter()
            .all(|index| usize::from(*index) < DIGITAL_RGBI_COLOR_COUNT),
        "title artwork color indices must fit the 16-color palette"
    );
    Ok(())
}

fn require_visible_pixels_fit_content_transfers(
    screen_color_indices: &[u8],
    background_color_indices: &[u8],
) -> Result<()> {
    for y in 0..TITLE_SCREEN_HEIGHT {
        for x in 0..TITLE_SCREEN_WIDTH {
            let index = screen_color_indices[y * TITLE_SCREEN_WIDTH + x];
            ensure!(
                background_color_indices.contains(&index)
                    || TITLE_CONTENT_TRANSFERS
                        .iter()
                        .any(|transfer| transfer.contains(x, y)),
                "title artwork has a visible pixel outside its consumer transfers at ({x}, {y})"
            );
        }
    }
    Ok(())
}

fn copy_transfer(
    source: &[u8],
    destination: &mut [Vec<u8>; 4],
    transfer: TitleTransfer,
) -> Result<()> {
    copy_compact_brgi(
        source,
        destination,
        transfer.source_offset,
        transfer.destination_offset,
        transfer.width_bytes,
        transfer.height,
    )
}

fn copy_compact_brgi(
    source: &[u8],
    destination: &mut [Vec<u8>; 4],
    source_offset: usize,
    destination_offset: usize,
    width_bytes: usize,
    height: usize,
) -> Result<()> {
    let plane_size = width_bytes
        .checked_mul(height)
        .context("title transfer plane size overflow")?;
    ensure!(
        source_offset + plane_size * 4 <= source.len(),
        "title transfer exceeds TITLE.DAT"
    );
    for (plane, destination_plane) in destination.iter_mut().enumerate() {
        for row in 0..height {
            let source_start = source_offset + plane * plane_size + row * width_bytes;
            let destination_start = destination_offset + row * SCREEN_ROW_BYTES;
            let destination_end = destination_start + width_bytes;
            ensure!(
                destination_end <= SCREEN_PLANE_BYTES,
                "title transfer crosses graphics VRAM"
            );
            destination_plane[destination_start..destination_end]
                .copy_from_slice(&source[source_start..source_start + width_bytes]);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "title_screen_tests.rs"]
mod title_screen_tests;
