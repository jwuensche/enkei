use crate::outputs::Mode;
use std::collections::HashMap;

use log::debug;

use crate::image::{
    error::ImageError,
    scaling::{Filter, Scaling},
    Image,
};

pub struct ResourceLoader {
    loaded: HashMap<String, Image>,
    scaled: HashMap<(String, Mode), Vec<u8>>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
            scaled: HashMap::new(),
        }
    }

    pub fn load(
        &mut self,
        path: &str,
        mode: &Mode,
        scaling: Scaling,
        filter: Filter,
    ) -> Result<&Vec<u8>, ImageError> {
        // workaround as this introduces nastier non-lexical lifetimes
        if self.scaled.contains_key(&(path.to_string(), *mode)) {
            // The scaling and filter cannot differ
            debug!(
                "Fetching scaled image from cache {{ path: {}, mode: {:?} }}",
                path, mode
            );
            return Ok(self.scaled.get(&(path.to_string(), *mode)).unwrap());
        }

        if !self.loaded.contains_key(path) {
            let surface = Image::new(path, scaling, filter)?;
            debug!("Caching image {{ path: {} }}", path);
            self.loaded.insert(path.to_string(), surface);
        }

        let surface = self.loaded.get(path).expect("Cannot fail");
        let surface_scaled = surface.process(mode)?;
        self.scaled
            .insert((path.to_string(), *mode), surface_scaled);
        return Ok(self
            .scaled
            .get(&(path.to_string(), *mode))
            .expect("Insertion was somehow misreported."));
    }
}
