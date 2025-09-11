/// Represents a bounding box for coordinate wrapping.
#[derive(Debug, Clone, Copy)] // Copy for convenience
pub struct BoxCoords {
    pub lat1: f64, // Effective min latitude
    pub lng1: f64, // Effective min longitude
    pub lat2: f64, // Effective max latitude
    pub lng2: f64, // Effective max longitude
}

impl std::str::FromStr for BoxCoords {
    type Err = anyhow::Error; // Use anyhow for robust error handling

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 4 {
            return Err(anyhow::anyhow!(
                "Invalid box format. Expected 'lat1,lng1,lat2,lng2', got '{}'",
                s
            ));
        }

        let lat1_input = parts[0].parse::<f64>()?;
        let lng1_input = parts[1].parse::<f64>()?;
        let lat2_input = parts[2].parse::<f64>()?;
        let lng2_input = parts[3].parse::<f64>()?;

        // Ensure lat1 < lat2 and lng1 < lng2 for consistent range calculations
        let (min_lat, max_lat) = if lat1_input < lat2_input {
            (lat1_input, lat2_input)
        } else {
            (lat2_input, lat1_input)
        };
        let (min_lng, max_lng) = if lng1_input < lng2_input {
            (lng1_input, lng2_input)
        } else {
            (lng2_input, lng1_input)
        };

        if min_lat == max_lat || min_lng == max_lng {
            return Err(anyhow::anyhow!(
                "Invalid box coordinates. Latitude or longitude range cannot be zero. Got lat1={}, lat2={} and lng1={}, lng2={}",
                lat1_input,
                lat2_input,
                lng1_input,
                lng2_input
            ));
        }

        Ok(BoxCoords {
            lat1: min_lat,
            lng1: min_lng,
            lat2: max_lat,
            lng2: max_lng,
        })
    }
}

// Add a helper function for coordinate wrapping
impl BoxCoords {
    pub fn wrap_latitude(&self, lat: f64) -> f64 {
        Self::wrap_coordinate(lat, self.lat1, self.lat2)
    }

    pub fn wrap_longitude(&self, lng: f64) -> f64 {
        Self::wrap_coordinate(lng, self.lng1, self.lng2)
    }

    // Helper function for wrapping a single coordinate within a [min, max) range
    fn wrap_coordinate(value: f64, min: f64, max: f64) -> f64 {
        let range_len = max - min;
        // Defensive check, though FromStr should prevent range_len from being 0
        if range_len == 0.0 {
            log::warn!(
                "Attempted to wrap coordinate with a zero range_len. Returning original value."
            );
            return value;
        }
        // Normalize to be relative to min, apply Euclidean modulo, then shift back.
        // f64::rem_euclid ensures the result is always in [0.0, range_len)
        (value - min).rem_euclid(range_len) + min
    }
}
