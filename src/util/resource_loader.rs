use std::path::PathBuf;

use crate::outputs::Mode;

use log::debug;

use crate::image::{
    error::ImageError,
    scaling::{Filter, Scaling},
    Image,
};

use cached::stores::SizedCache;
use cached::Cached;

pub struct ResourceLoader {
    last_loaded: SizedCache<PathBuf, Image>,
    scaled: SizedCache<(PathBuf, Mode), Vec<u8>>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        Self {
            last_loaded: SizedCache::with_size(2),
            scaled: SizedCache::with_size(2),
        }
    }

    // TODO: Declonify this by changing the keys
    pub fn load(
        &mut self,
        path: &PathBuf,
        mode: &Mode,
        scaling: Scaling,
        filter: Filter,
    ) -> Result<&Vec<u8>, ImageError> {
        // workaround as this introduces nastier non-lexical lifetimes
        if self.scaled.cache_get(&(path.clone(), *mode)).is_some() {
            // The scaling and filter cannot differ
            debug!(
                "Fetching scaled image from cache {{ path: {:?}, mode: {:?} }}",
                path, mode
            );
            return Ok(self.scaled.cache_get(&(path.clone(), *mode)).unwrap());
        }

        if self.last_loaded.cache_get(&path).is_none() {
            let surface = Image::new(path.clone(), scaling, filter)?;
            debug!("Caching image {{ path: {:?} }}", path);
            self.last_loaded.cache_set(path.clone(), surface);
        }

        let surface = self.last_loaded.cache_get(&path).expect("Cannot fail");
        let surface_scaled = surface.process(mode)?;
        self.scaled.cache_set((path.clone(), *mode), surface_scaled);
        return Ok(self
            .scaled
            .cache_get(&(path.clone(), *mode))
            .expect("Insertion was somehow misreported."));
    }
}
