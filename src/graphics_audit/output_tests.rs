use super::*;
use crate::graphics_audit::planar::RgbImage;

#[test]
fn publishing_refuses_an_existing_directory() {
    let directory = tempfile::tempdir().unwrap();

    let error = publish_previews(directory.path(), Vec::new()).unwrap_err();

    assert!(error.to_string().contains("refusing to overwrite"));
}

#[test]
fn png_writer_emits_the_requested_rgb_dimensions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("preview.png");
    let image = RgbImage {
        width: 2,
        height: 1,
        pixels: vec![255, 0, 0, 0, 255, 0],
    };

    write_png(&path, &image).unwrap();

    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).unwrap()));
    let reader = decoder.read_info().unwrap();
    assert_eq!(reader.info().width, 2);
    assert_eq!(reader.info().height, 1);
}
