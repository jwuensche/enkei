use crate::outputs::Mode;
use std::collections::HashMap;

use log::debug;

use crate::image::{
    error::ImageError,
    scaling::{Filter, Scaling},
    Image,
};

use cached::stores::SizedCache;
use cached::Cached;

pub struct ResourceLoader {
    last_loaded: SizedCache<String, Image>,
    scaled: SizedCache<(String, Mode), Vec<u8>>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        Self {
            last_loaded: SizedCache::with_size(2),
            scaled: SizedCache::with_size(2),
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
        if self.scaled.cache_get(&(path.to_string(), *mode)).is_some() {
            // The scaling and filter cannot differ
            debug!(
                "Fetching scaled image from cache {{ path: {}, mode: {:?} }}",
                path, mode
            );
            return Ok(self.scaled.cache_get(&(path.to_string(), *mode)).unwrap());
        }

        // uGH DiSgUsTiNg
         if !self.last_loaded.cache_get(&path.to_string()).is_some() {
            let surface = Image::new(path, scaling, filter)?;
             debug!("Caching image {{ path: {} }}", path);
             self.last_loaded.cache_set(path.to_string(), surface);
        }

        let surface = self.last_loaded.cache_get(&path.to_string()).expect("Cannot fail");
        let surface_scaled = surface.process(mode)?;
        self.scaled
            .cache_set((path.to_string(), *mode), surface_scaled);
        return Ok(self
            .scaled
            .cache_get(&(path.to_string(), *mode))
            .expect("Insertion was somehow misreported."));
    }
}
