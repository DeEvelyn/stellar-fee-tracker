use stellar_devkit::utilities::converters::{
    xlm_to_stroop, stroop_to_xlm, ConversionError, STROOPS_PER_XLM,
};

#[test]
fn stroop_to_xlm_basic() {
    assert_eq!(stroop_to_xlm(0), 0.0);
    assert_eq!(stroop_to_xlm(STROOPS_PER_XLM), 1.0);
    assert_eq!(stroop_to_xlm(STROOPS_PER_XLM * 5), 5.0);
}

#[test]
fn xlm_to_stroop_basic() {
    assert_eq!(xlm_to_stroop(0.0).unwrap(), 0);
    assert_eq!(xlm_to_stroop(1.0).unwrap(), STROOPS_PER_XLM);
    assert_eq!(xlm_to_stroop(5.0).unwrap(), STROOPS_PER_XLM * 5);
}

#[test]
fn roundtrip_accuracy() {
    let stroops = 123_456_789u64;
    let xlm = stroop_to_xlm(stroops);
    let back = xlm_to_stroop(xlm).unwrap();
    assert_eq!(stroops, back);
}

#[test]
fn roundtrip_fractional() {
    let xlm = 1.5;
    let stroops = xlm_to_stroop(xlm).unwrap();
    let back = stroop_to_xlm(stroops);
    assert!((back - xlm).abs() < 1e-7);
}

#[test]
fn negative_xlm_returns_error() {
    assert_eq!(xlm_to_stroop(-1.0), Err(ConversionError::NegativeXlm));
}

#[test]
fn boundary_zero() {
    assert_eq!(xlm_to_stroop(0.0).unwrap(), 0);
    assert_eq!(stroop_to_xlm(0), 0.0);
}

#[test]
fn large_value() {
    let xlm = 1_000_000.0;
    let stroops = xlm_to_stroop(xlm).unwrap();
    assert_eq!(stroops, 10_000_000_000_000);
    assert_eq!(stroop_to_xlm(stroops), xlm);
}

#[test]
fn conversion_error_display() {
    let err = ConversionError::NegativeXlm;
    assert!(err.to_string().contains("non-negative"));
}
