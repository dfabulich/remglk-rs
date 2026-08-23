/*

Glk Mapping extension
=====================

Copyright (c) 2026
MIT licenced

*/

use std::collections::HashMap;

use super::*;

const MAP_MAX_OVERLAYS: u32 = 256;
const MAP_MAX_HYPERLINKS: u32 = 64;
const MAP_MAX_POINTS_PER_LINK: u32 = 32;

#[derive(Clone, Default)]
pub struct MapDocument {
    pub format: MapFormat,
    pub data: Option<String>,
    pub image: Option<u32>,
    pub bgcolor: Option<String>,
    pub focus: Option<MapFocus>,
    pub hyperlinks: Vec<MapHyperlink>,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum MapFormat {
    #[default]
    None,
    Svg,
    Image,
}

#[derive(Clone)]
pub struct MapState {
    pub doc: Option<MapDocument>,
    pub overlays: HashMap<u32, MapOverlay>,
    pub next_overlay_id: u32,
    pub pending: Option<MapUpdate>,
    pub changed: bool,
    pub event_pending: bool,
}

impl Default for MapState {
    fn default() -> Self {
        Self {
            next_overlay_id: 1,
            doc: None,
            overlays: HashMap::new(),
            pending: None,
            changed: false,
            event_pending: false,
        }
    }
}

impl MapState {
    fn queue(&mut self, update: MapUpdate) {
        self.pending = Some(match self.pending.take() {
            Some(mut prev) => {
                merge_map_update(&mut prev, update);
                prev
            },
            None => update,
        });
        self.changed = true;
    }

    fn clear_overlays_local(&mut self) {
        self.overlays.clear();
    }

    fn alloc_overlay_id(&mut self) -> u32 {
        for _ in 0..MAP_MAX_OVERLAYS {
            let id = self.next_overlay_id;
            self.next_overlay_id = self.next_overlay_id.wrapping_add(1);
            if id == 0 {
                continue;
            }
            if !self.overlays.contains_key(&id) {
                return id;
            }
        }
        0
    }

    pub fn filter_hyperlinks(hyperlinks: &[MapHyperlinkArg]) -> Vec<MapHyperlink> {
        hyperlinks
            .iter()
            .take(MAP_MAX_HYPERLINKS as usize)
            .filter(|h| {
                h.id != 0
                    && h.points.len() >= 3
                    && h.points.len() <= MAP_MAX_POINTS_PER_LINK as usize
            })
            .map(|h| MapHyperlink {
                id: h.id,
                label: h.label.clone(),
                points: h.points.clone(),
            })
            .collect()
    }

    fn focus_from_args(
        focusleft: i32,
        focustop: i32,
        focuswidth: u32,
        focusheight: u32,
    ) -> Option<MapFocus> {
        if focuswidth == 0 || focusheight == 0 {
            return None;
        }
        Some(MapFocus {
            left: focusleft,
            top: focustop,
            width: focuswidth,
            height: focusheight,
        })
    }
}

pub struct MapHyperlinkArg {
    pub id: u32,
    pub label: Option<String>,
    pub points: Vec<MapPoint>,
}

fn merge_map_update(dest: &mut MapUpdate, src: MapUpdate) {
    if src.clear {
        *dest = MapUpdate {
            clear: true,
            ..Default::default()
        };
        return;
    }
    if src.present.is_some() {
        dest.present = src.present;
        // A new present replaces the prior document; drop staged overlays.
        dest.overlays.clear();
        dest.overlay_moves.clear();
        dest.overlay_clears.clear();
        dest.overlay_clear_all = true;
    }
    if src.focus.is_some() {
        dest.focus = src.focus;
    }
    if src.hyperlinks.is_some() {
        dest.hyperlinks = src.hyperlinks;
    }
    if src.overlay_clear_all {
        dest.overlay_clear_all = true;
        dest.overlays.clear();
        dest.overlay_moves.clear();
        dest.overlay_clears.clear();
    }
    dest.overlays.extend(src.overlays);
    dest.overlay_moves.extend(src.overlay_moves);
    dest.overlay_clears.extend(src.overlay_clears);
}

impl<S> GlkApi<S>
where S: Default + GlkSystem {
    pub fn glk_map_present_svg(
        &mut self,
        data: &[u8],
        bgcolor: u32,
        focusleft: i32,
        focustop: i32,
        focuswidth: u32,
        focusheight: u32,
        hyperlinks: &[MapHyperlinkArg],
    ) -> u32 {
        if !self.support.map {
            return 0;
        }
        let Ok(svg) = std::str::from_utf8(data) else {
            return 0;
        };
        if !svg.contains("<svg") && !svg.contains("<SVG") {
            return 0;
        }
        let focus = MapState::focus_from_args(focusleft, focustop, focuswidth, focusheight);
        let links = MapState::filter_hyperlinks(hyperlinks);
        let bgcolor_css = map_bgcolor_css(bgcolor);
        self.map.clear_overlays_local();
        self.map.doc = Some(MapDocument {
            format: MapFormat::Svg,
            data: Some(svg.to_string()),
            bgcolor: bgcolor_css.clone(),
            focus: focus.clone(),
            hyperlinks: links.clone(),
            ..Default::default()
        });
        self.map.queue(MapUpdate {
            overlay_clear_all: true,
            present: Some(MapPresent {
                format: "svg",
                data: Some(svg.to_string()),
                image: None,
                bgcolor: bgcolor_css,
                focus,
                hyperlinks: Some(links),
            }),
            ..Default::default()
        });
        1
    }

    pub fn glk_map_present_image(
        &mut self,
        image: u32,
        bgcolor: u32,
        focusleft: i32,
        focustop: i32,
        focuswidth: u32,
        focusheight: u32,
        hyperlinks: &[MapHyperlinkArg],
    ) -> u32 {
        if !self.support.map || image == 0 {
            return 0;
        }
        if self.glk_image_get_info(image).is_none() {
            return 0;
        }
        let focus = MapState::focus_from_args(focusleft, focustop, focuswidth, focusheight);
        let links = MapState::filter_hyperlinks(hyperlinks);
        let bgcolor_css = map_bgcolor_css(bgcolor);
        self.map.clear_overlays_local();
        self.map.doc = Some(MapDocument {
            format: MapFormat::Image,
            image: Some(image),
            bgcolor: bgcolor_css.clone(),
            focus: focus.clone(),
            hyperlinks: links.clone(),
            ..Default::default()
        });
        self.map.queue(MapUpdate {
            overlay_clear_all: true,
            present: Some(MapPresent {
                format: "image",
                data: None,
                image: Some(image),
                bgcolor: bgcolor_css,
                focus,
                hyperlinks: Some(links),
            }),
            ..Default::default()
        });
        1
    }

    pub fn glk_map_set_hyperlinks(&mut self, hyperlinks: &[MapHyperlinkArg]) {
        let links = MapState::filter_hyperlinks(hyperlinks);
        if let Some(doc) = self.map.doc.as_mut() {
            doc.hyperlinks = links.clone();
        }
        self.map.queue(MapUpdate {
            hyperlinks: Some(links),
            ..Default::default()
        });
    }

    pub fn glk_map_overlay(
        &mut self,
        image: u32,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        zindex: u32,
        link_id: u32,
        link_label: Option<String>,
    ) -> u32 {
        if self.map.doc.is_none() || image == 0 || self.glk_image_get_info(image).is_none() {
            return 0;
        }
        let id = self.map.alloc_overlay_id();
        if id == 0 {
            return 0;
        }
        let ov = MapOverlay {
            id,
            image: Some(image),
            left,
            top,
            width,
            height,
            zindex,
            link_id: if link_id == 0 { None } else { Some(link_id) },
            link_label,
            ..Default::default()
        };
        self.map.overlays.insert(id, ov.clone());
        self.map.queue(MapUpdate {
            overlays: vec![ov],
            ..Default::default()
        });
        id
    }

    pub fn glk_map_overlay_svg(
        &mut self,
        data: &[u8],
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        zindex: u32,
        link_id: u32,
        link_label: Option<String>,
    ) -> u32 {
        if self.map.doc.is_none() {
            return 0;
        }
        let Ok(svg) = std::str::from_utf8(data) else {
            return 0;
        };
        if !svg.contains("<svg") && !svg.contains("<SVG") {
            return 0;
        }
        if self.map.overlays.len() >= MAP_MAX_OVERLAYS as usize {
            return 0;
        }
        let id = self.map.alloc_overlay_id();
        if id == 0 {
            return 0;
        }
        let ov = MapOverlay {
            id,
            svg: Some(svg.to_string()),
            left,
            top,
            width,
            height,
            zindex,
            link_id: if link_id == 0 { None } else { Some(link_id) },
            link_label,
            ..Default::default()
        };
        self.map.overlays.insert(id, ov.clone());
        self.map.queue(MapUpdate {
            overlays: vec![ov],
            ..Default::default()
        });
        id
    }

    pub fn glk_map_fill_rect(
        &mut self,
        color: u32,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        zindex: u32,
    ) -> u32 {
        if self.map.doc.is_none() || width == 0 || height == 0 {
            return 0;
        }
        let id = self.map.alloc_overlay_id();
        if id == 0 {
            return 0;
        }
        let ov = MapOverlay {
            id,
            color: Some(colour_code_to_css(color)),
            left,
            top,
            width,
            height,
            zindex,
            ..Default::default()
        };
        self.map.overlays.insert(id, ov.clone());
        self.map.queue(MapUpdate {
            overlays: vec![ov],
            ..Default::default()
        });
        id
    }

    pub fn glk_map_overlay_move(
        &mut self,
        overlay: u32,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        zindex: u32,
    ) -> u32 {
        let Some(ov) = self.map.overlays.get_mut(&overlay) else {
            return 0;
        };
        ov.left = left;
        ov.top = top;
        ov.width = width;
        ov.height = height;
        ov.zindex = zindex;
        self.map.queue(MapUpdate {
            overlay_moves: vec![MapOverlayMove {
                id: overlay,
                left,
                top,
                width,
                height,
                zindex,
            }],
            ..Default::default()
        });
        1
    }

    pub fn glk_map_overlay_clear(&mut self, overlay: u32) -> u32 {
        if overlay == 0 || self.map.overlays.remove(&overlay).is_none() {
            return 0;
        }
        self.map.queue(MapUpdate {
            overlay_clears: vec![overlay],
            ..Default::default()
        });
        1
    }

    pub fn glk_map_overlay_clear_all(&mut self) -> u32 {
        self.map.clear_overlays_local();
        self.map.queue(MapUpdate {
            overlay_clear_all: true,
            ..Default::default()
        });
        1
    }

    pub fn glk_map_close(&mut self) {
        self.map.doc = None;
        self.map.clear_overlays_local();
        self.map.event_pending = false;
        self.map.queue(MapUpdate {
            clear: true,
            ..Default::default()
        });
    }

    pub fn glk_map_get_visibility(&self) -> u32 {
        if self.map.doc.is_some() { 1 } else { 0 }
    }

    pub fn glk_map_show_at_user_request(&mut self) {
        // Harness has no user-hide latch; map content is already shown on present.
    }

    pub fn glk_request_map_event(&mut self) {
        if self.support.map {
            self.map.event_pending = true;
        }
    }

    pub fn glk_cancel_map_event(&mut self) {
        self.map.event_pending = false;
    }

    pub fn glk_map_set_focus(
        &mut self,
        focusleft: i32,
        focustop: i32,
        focuswidth: u32,
        focusheight: u32,
    ) {
        let focus = MapFocus {
            left: focusleft,
            top: focustop,
            width: focuswidth,
            height: focusheight,
        };
        if let Some(doc) = self.map.doc.as_mut() {
            doc.focus = Some(focus.clone());
        }
        self.map.queue(MapUpdate {
            focus: Some(Some(focus)),
            ..Default::default()
        });
    }

    pub fn glk_map_clear_focus(&mut self) {
        if let Some(doc) = self.map.doc.as_mut() {
            doc.focus = None;
        }
        self.map.queue(MapUpdate {
            focus: Some(None),
            ..Default::default()
        });
    }
}

fn map_bgcolor_css(bgcolor: u32) -> Option<String> {
    if bgcolor == mapcolor_Default {
        None
    } else {
        Some(colour_code_to_css(bgcolor))
    }
}
