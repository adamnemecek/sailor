use wgpu::RenderPipeline;
use core::ops::Range;
use crate::drawing::feature_collection::FeatureCollection;
use crate::vector_tile::math::TileId;
use wgpu::{
    RenderPass,
    Buffer,
    Device,
};

use crate::vector_tile::tile::Tile;

use pathfinder_renderer::scene::{PathObject, Scene};

pub struct DrawableTile {
    pub tile_id: TileId,
    pub index_count: u32,
    pub features: Vec<(u32, Range<u32>)>,
    pub extent: u16,
    pub scene: Scene,
}

impl DrawableTile {
    pub fn load_from_tile_id(
        tile_id: TileId,
        tile: &Tile,
    ) -> DrawableTile {
        let mut features = Vec::new();
        for l in tile.layers.clone() {
            features.extend(l.features);
        }

        DrawableTile {
            index_count: tile.mesh.indices.len() as u32,
            features,
            tile_id,
            extent: tile.extent,
            scene: tile.scene.clone(),
        }
    }

    pub fn paint(
        &mut self,
        render_pass: &mut RenderPass,
        blend_pipeline: &RenderPipeline,
        noblend_pipeline: &RenderPipeline,
        feature_collection: &FeatureCollection,
        tile_id: u32
    ) {
        let mut alpha_set = vec![];
        let mut opaque_set = vec![];

        self.features.sort_by(|a, b| {
            feature_collection
                .get_zindex(a.0)
                .partial_cmp(&feature_collection.get_zindex(b.0)).unwrap()
        });

        for (id, range) in &self.features {
            if feature_collection.has_alpha(*id) {
                alpha_set.push((id, range));
            } else {
                opaque_set.push((id, range));
            }
        }

        let mut i = 0;
        render_pass.set_pipeline(noblend_pipeline);
        for (id, range) in opaque_set {
            if range.len() > 0 && feature_collection.is_visible(*id) {
                render_pass.set_stencil_reference(i as u32);
                i += 1;

                let range_start = (tile_id << 1) | 1;
                render_pass.draw_indexed(range.clone(), 0, 0 + range_start .. 1 + range_start);

                if feature_collection.has_outline(*id) {
                    let range_start = tile_id << 1;
                    render_pass.draw_indexed(range.clone(), 0, 0 + range_start .. 1 + range_start);
                }
            }
        }

        let mut i = 0;
        render_pass.set_pipeline(blend_pipeline);
        for (id, range) in alpha_set {
            if range.len() > 0 && feature_collection.is_visible(*id) {
                render_pass.set_stencil_reference(i as u32);
                i += 1;

                let range_start = (tile_id << 1) | 1;
                render_pass.draw_indexed(range.clone(), 0, 0 + range_start .. 1 + range_start);

                if feature_collection.has_outline(*id) {
                    let range_start = tile_id << 1;
                    render_pass.draw_indexed(range.clone(), 0, 0 + range_start .. 1 + range_start);
                }
            }
        }
    }
}