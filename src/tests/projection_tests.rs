use super::*;
use crate::error::ApiError;
use crate::projection::ProjectionTransformer;

#[test]
fn test_validate_valid_srids() {
    let valid_srids = vec![
        4326,  // WGS84
        3857,  // Web Mercator
        2163,  // US National Atlas Equal Area
        3005,  // BC Albers
        26907, // UTM Zone 7N NAD83
        26908, // UTM Zone 8N NAD83
        26909, // UTM Zone 9N NAD83
        26910, // UTM Zone 10N NAD83
        26911, // UTM Zone 11N NAD83
        3157,  // NAD83(CSRS) / BC Albers
        4269,  // NAD83 Geographic
        4617,  // NAD83(CSRS) Geographic
        3155,  // NAD83(CSRS) / UTM Zone 7N
        3156,  // NAD83(CSRS) / UTM Zone 8N
        2955,  // NAD83(CSRS) / UTM Zone 9N
        3158,  // NAD83(CSRS) / UTM Zone 10N
        3159,  // NAD83(CSRS) / UTM Zone 11N
    ];

    for srid in valid_srids {
        assert!(ProjectionTransformer::validate_srid(srid).is_ok());
    }
}

#[test]
fn test_validate_invalid_srid() {
    let result = ProjectionTransformer::validate_srid(1234);
    match result {
        Err(ApiError::InvalidCoordinateSystem { srid, message, supported_systems }) => {
            assert_eq!(srid, 1234);
            assert_eq!(message, "Unsupported coordinate system");
            assert!(supported_systems.contains(&4326));
            assert!(supported_systems.contains(&3857));
        },
        _ => panic!("Expected InvalidCoordinateSystem error"),
    }
} 