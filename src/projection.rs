use geojson::Feature;
use crate::error::ApiError;

pub struct ProjectionTransformer;

impl ProjectionTransformer {
    const VALID_SRIDS: &'static [i32] = &[
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

    // Transform feature geometry from input SRID to WGS84
    #[allow(dead_code)]
    pub fn transform_feature(feature: &Feature, input_srid: i32) -> Result<Feature, ApiError> {
        if input_srid == 4326 {
            return Ok(feature.clone());
        }

        Self::validate_srid(input_srid)?;

        // Clone the feature and transform its geometry
        let mut transformed = feature.clone();
        
        // Transform geometry to WGS84 (SRID 4326)
        // Note: This is a placeholder. In a real implementation,
        // you would use a proper coordinate transformation library
        transformed.geometry = match feature.geometry.clone() {
            Some(geom) => Some(geom),
            None => return Err(ApiError::InvalidGeometry("Missing geometry".to_string())),
        };

        Ok(transformed)
    }

    // Validate SRID against supported coordinate systems
    pub fn validate_srid(srid: i32) -> Result<(), ApiError> {
        if !Self::VALID_SRIDS.contains(&srid) {
            return Err(ApiError::InvalidCoordinateSystem {
                srid,
                message: "Unsupported coordinate system".to_string(),
                supported_systems: Self::VALID_SRIDS.to_vec(),
            });
        }
        Ok(())
    }
} 