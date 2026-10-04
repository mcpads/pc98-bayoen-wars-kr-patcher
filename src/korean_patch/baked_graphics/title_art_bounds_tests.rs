use super::*;

#[test]
fn source_sized_and_centered_replacement_is_accepted() {
    let source = VisibleTitleBounds {
        x: 70,
        y: 13,
        width: 500,
        height: 307,
    };
    let replacement = VisibleTitleBounds {
        x: 72,
        y: 14,
        width: 490,
        height: 306,
    };

    validate_replacement_title_bounds(source, replacement).unwrap();
}

#[test]
fn undersized_replacement_is_rejected() {
    let source = VisibleTitleBounds {
        x: 70,
        y: 13,
        width: 500,
        height: 307,
    };
    let replacement = VisibleTitleBounds {
        x: 120,
        y: 46,
        width: 454,
        height: 244,
    };

    let error = validate_replacement_title_bounds(source, replacement).unwrap_err();
    assert!(error.to_string().contains("visible width"));
}

#[test]
fn displaced_replacement_is_rejected() {
    let source = VisibleTitleBounds {
        x: 70,
        y: 13,
        width: 500,
        height: 307,
    };
    let replacement = VisibleTitleBounds {
        x: 90,
        y: 13,
        width: 500,
        height: 307,
    };

    let error = validate_replacement_title_bounds(source, replacement).unwrap_err();
    assert!(error.to_string().contains("visible center"));
}
