use pathfinder_renderer::concurrent::{
    rayon::RayonExecutor,
    scene_proxy::SceneProxy,
};
use crate::drawing::feature_collection::FeatureCollection;
use std::sync::RwLock;
use std::sync::Arc;
use crate::drawing::drawable_tile::DrawableTile;
use crate::vector_tile::math::TileId;
use nom::lib::std::collections::BTreeMap;
use crate::app_state::AppState;

use crate::config::CONFIG;

use pathfinder_content::color::ColorF;
use sdl2::video::GLProfile;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_gpu::resources::FilesystemResourceLoader;
use pathfinder_renderer::options::BuildOptions;

pub struct PathfinderPainter {
    pub renderer: Renderer<GLDevice>,
    pub window: sdl2::video::Window,
    pub window_size: Vector2I,
    pub loaded_tiles: BTreeMap<TileId, DrawableTile>,
    pub feature_collection: Arc<RwLock<FeatureCollection>>,
}

impl PathfinderPainter {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video = sdl_context.video().unwrap();

        // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
        let gl_attributes = video.gl_attr();
        gl_attributes.set_context_profile(GLProfile::Core);
        gl_attributes.set_context_version(3, 3);

        // Open a window.
        let window_size = Vector2I::new(640, 480);
        let window = video.window("Minimal example", window_size.x() as u32, window_size.y() as u32)
                        .opengl()
                        .build()
                        .unwrap();

        // Create the GL context, and make it current.
        let gl_context = window.gl_create_context().unwrap();
        gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
        window.gl_make_current(&gl_context).unwrap();

        // Create a Pathfinder renderer.
        let mut renderer = Renderer::new(
            GLDevice::new(GLVersion::GL3, 0),
            &FilesystemResourceLoader::locate(),
            DestFramebuffer::full_window(window_size),
            RendererOptions { background_color: Some(ColorF::white()) }
        );

        Self {
            renderer,
            window,
            window_size,
            loaded_tiles: BTreeMap::new(),
            feature_collection: Arc::new(RwLock::new(FeatureCollection::new(1000)))
        }
    }

    fn load_tiles(&mut self, app_state: &mut AppState) {
        let tile_field = app_state.screen.get_tile_boundaries_for_zoom_level(app_state.zoom, 1);

        // Remove old bigger tiles which are not in the FOV anymore.
        let old_tile_field = app_state.screen.get_tile_boundaries_for_zoom_level(app_state.zoom - 1.0, 2);
        let key_iter: Vec<_> = self.loaded_tiles.keys().copied().collect();
        for key in key_iter {
            if key.z == (app_state.zoom - 1.0) as u32 {
                if !old_tile_field.contains(&key) {
                    self.loaded_tiles.remove(&key);
                }
            } else {
                if !tile_field.contains(&key) {
                    self.loaded_tiles.remove(&key);
                }
            }
        }

        app_state.tile_cache.fetch_tiles();
                
                let tile_cache = &mut app_state.tile_cache;
        for tile_id in tile_field.iter() {
            if !self.loaded_tiles.contains_key(&tile_id) {
                tile_cache.request_tile(&tile_id, self.feature_collection.clone());
                if let Some(tile) = tile_cache.try_get_tile(&tile_id) {

                    let drawable_tile = DrawableTile::load_from_tile_id(
                        tile_id,
                        &tile,
                    );

                    self.loaded_tiles.insert(
                        tile_id.clone(),
                        drawable_tile
                    );

                    // Remove old bigger tile when all 4 smaller tiles are loaded.
                    let mut count = 0;
                    let num_x = (tile_id.x / 2) * 2;
                    let num_y = (tile_id.y / 2) * 2;
                    for tile_id in &[
                        TileId::new(tile_id.z, num_x, num_y),
                        TileId::new(tile_id.z, num_x + 1, num_y),
                        TileId::new(tile_id.z, num_x + 1, num_y + 1),
                        TileId::new(tile_id.z, num_x, num_y + 1),
                    ] {
                        if !tile_field.contains(tile_id) {
                            count += 1;
                            continue;
                        }
                        if self.loaded_tiles.contains_key(tile_id) {
                            count += 1;
                        }
                    }
                    if count == 4 {
                        self.loaded_tiles.remove(&TileId::new(tile_id.z - 1, num_x / 2, num_y / 2));
                    }

                    // Remove old smaller tiles when all 4 smaller tiles are loaded.
                    for tile_id in &[
                        TileId::new(tile_id.z + 1, tile_id.x * 2, tile_id.y * 2),
                        TileId::new(tile_id.z + 1, tile_id.x * 2 + 1, tile_id.y * 2),
                        TileId::new(tile_id.z + 1, tile_id.x * 2 + 1, tile_id.y * 2 + 1),
                        TileId::new(tile_id.z + 1, tile_id.x * 2, tile_id.y * 2 + 1),
                    ] {
                        self.loaded_tiles.remove(tile_id);
                    }
                } else {
                    log::trace!("Could not read tile {} from cache.", tile_id);
                }
            }
        }

        let mut feature_collection = self.feature_collection.write().unwrap();
        feature_collection.load_styles(app_state.zoom, &mut app_state.css_cache);
    }

    pub fn paint(&mut self, app_state: &mut AppState) {
        for (i, dt) in self.loaded_tiles.values_mut().enumerate() {
            SceneProxy::from_scene(dt.scene.clone(), RayonExecutor).build_and_render(&mut self.renderer, BuildOptions::default());
        }
        self.window.gl_swap_window();
    }
}