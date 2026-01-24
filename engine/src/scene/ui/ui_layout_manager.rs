use crate::math::*;
use crate::scene::scene::{Entity, Scene};
use crate::scene::ui::{Alignment, AnchorPoint, DockMode, Offset, PackingMode, Padding, Size, StackDirection};
use crate::scene::ui::layout::{Container, ParentRelation, UiNodeLayout};

impl Scene {
    pub(crate) fn layout_ui(&mut self) {
        let window_size = (
            self.context.window.inner_size().width as f32,
            self.context.window.inner_size().height as f32
        );

        let root_entities = self.ui_root_entities.clone();
        for root_node_index in root_entities {
            self.layout_node(
                &window_size,
                root_node_index,
                &(0.0, 0.0),
                &window_size,
                &((0.0, 0.0), window_size),
            );
        }
    }
    fn layout_node(
        &mut self,
        gui_viewport: &(f32, f32),
        node_index: usize,
        parent_origin: &(f32, f32),
        parent_size: &(f32, f32),
        parent_clipping: &((f32, f32), (f32, f32)),
    ) {
        let entity = &mut self.entities[node_index];
        let layout = &mut self.ui_node_layouts[entity.ui_layout.unwrap()];
        // set size and position if this container is independent, otherwise it was already set by the parent
        {
            if let Some(relation) = &layout.parent_relation {
                match relation {
                    ParentRelation::Independent { relative, anchor, offset_x, offset_y } => {
                        let anchor_factor = match anchor {
                            AnchorPoint::TopLeft => (0.0, 0.0),
                            AnchorPoint::TopCenter => (0.5, 0.0),
                            AnchorPoint::TopRight => (1.0, 0.0),
                            AnchorPoint::CenterLeft => (0.0, 0.5),
                            AnchorPoint::Center => (0.5, 0.5),
                            AnchorPoint::CenterRight => (1.0, 0.5),
                            AnchorPoint::BottomLeft => (0.0, 1.0),
                            AnchorPoint::BottomCenter => (0.5, 1.0),
                            AnchorPoint::BottomRight => (1.0, 1.0),
                        };
                        let final_parent_size = if *relative { parent_size } else { gui_viewport };

                        let node_size = Self::calculate_size(&layout.width, &layout.height, &final_parent_size);
                        layout.size = Vector::new2(node_size.0.0, node_size.1.0);

                        layout.position.x = parent_origin.0 + final_parent_size.0*anchor_factor.0 + match offset_x {
                            Offset::Pixels(p) => *p,
                            Offset::Factor(f) => *f * final_parent_size.0
                        };
                        layout.position.y = parent_origin.1 + final_parent_size.1*anchor_factor.1 + match offset_y {
                            Offset::Pixels(p) => *p,
                            Offset::Factor(f) => *f * final_parent_size.1
                        }
                    },
                    _ => ()
                }
            }
        }

        // fetch properties first to avoid borrow checker issues
        let container = layout.container.clone();
        let children_indices = entity.children_indices.clone();
        let node_pos = (layout.position.x, layout.position.y);
        let node_size = (layout.size.x, layout.size.y);
        let node_clipping = layout.clipping;

        let node_clip_bounds = if node_clipping {
            let clip_min = (
                node_pos.0.max(parent_clipping.0.0),
                node_pos.1.max(parent_clipping.0.1),
            );
            let clip_max = (
                (node_pos.0 + node_size.0).min(parent_clipping.1.0),
                (node_pos.1 + node_size.1).min(parent_clipping.1.1),
            );
            (clip_min, clip_max)
        } else {
            (node_pos, (node_pos.0 + node_size.0, node_pos.1 + node_size.1))
        };

        layout.clip_min.x = node_clip_bounds.0.0;
        layout.clip_min.y = node_clip_bounds.0.1;
        layout.clip_max.x = node_clip_bounds.1.0;
        layout.clip_max.y = node_clip_bounds.1.1;

        // layout children based on this container type
        match container {
            Container::Stack { horizontal, spacing, padding, packing, alignment, stack_direction } => {
                Self::layout_stack(
                    &mut self.entities,
                    &mut self.ui_node_layouts,
                    &children_indices,
                    node_pos,
                    node_size,
                    horizontal,
                    spacing,
                    padding,
                    packing,
                    alignment,
                    stack_direction,
                );
            },
            Container::Dock => {
                Self::layout_dock(
                    &mut self.entities,
                    &mut self.ui_node_layouts,
                    &children_indices,
                    node_pos,
                    node_size,
                );
            },
        }
        for child_index in children_indices.iter() {
            self.layout_node(
                gui_viewport,
                *child_index,
                &node_pos,
                &node_size,
                &node_clip_bounds,
            )
        }
    }
    fn calculate_size(
        width: &Size,
        height: &Size,
        parent_size: &(f32, f32),
    ) -> ((f32, bool), (f32, bool)) {
        let mut width_is_copy = false;
        let mut width = match width {
            Size::Absolute(p) => (*p, false),
            Size::Factor(f) => (parent_size.0 * *f, false),
            Size::FillFactor(f) => (*f, true), // recalculated in container
            Size::Auto => (100.0, false), // TODO: Calculate from content,
            Size::Copy => { width_is_copy = true;
                println!("e"); (parent_size.0, false) },
        };

        let height = match height {
            Size::Absolute(p) => (*p, false),
            Size::Factor(f) => (parent_size.1 * *f, false),
            Size::FillFactor(f) => (*f, true), // recalculated in container
            Size::Auto => (100.0, false), // TODO: Calculate from content,
            Size::Copy => {
                if width_is_copy {
                    panic!("Cannot have both width and height copy eachother")
                } else { width }
            }
        };

        if width_is_copy { width = height }

        (width, height)
    }
    fn layout_dock(
        entities: &mut Vec<Entity>,
        layouts: &mut Vec<UiNodeLayout>,
        children_indices: &Vec<usize>,
        node_position: (f32, f32),
        node_size: (f32, f32),
    ) {
        let operable_children_indices = children_indices.iter().filter_map(|&child_index| {
            let entity = &mut entities[child_index];
            let layout = &mut layouts[entity.ui_layout.unwrap()];
            if let Some(relation) = &layout.parent_relation {
                match relation {
                    ParentRelation::Independent { .. } => None,
                    ParentRelation::Docking(_) => Some(child_index),
                }
            } else {
                None
            }
        }).collect::<Vec<_>>();
        if operable_children_indices.is_empty() {
            return;
        }

        let mut remaining_space = node_size;
        let mut offset = (0.0, 0.0);

        for &child_index in operable_children_indices.iter() {
            let child = &entities[child_index];
            let layout = &mut layouts[child.ui_layout.unwrap()];

            let dock_mode;
            if let Some(relation) = &layout.parent_relation {
                match relation {
                    ParentRelation::Independent { .. } => continue,
                    ParentRelation::Docking(mode) => dock_mode = mode.clone(),
                }
            } else {
                continue
            }

            // child size based on remaining space
            let child_size = Self::calculate_size(&layout.width, &layout.height, &remaining_space);
            let child_size = (child_size.0.0, child_size.1.0);

            // position and update remaining space based on dock mode
            let (child_pos, child_final_size) = match dock_mode {
                DockMode::Top => {
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = (remaining_space.0, child_size.1);

                    offset.1 += child_size.1;
                    remaining_space.1 -= child_size.1;

                    (pos, size)
                },
                DockMode::Bottom => {
                    remaining_space.1 -= child_size.1;
                    let pos = (
                        node_position.0 + offset.0,
                        node_position.1 + offset.1 + remaining_space.1
                    );
                    let size = (remaining_space.0, child_size.1);

                    (pos, size)
                },
                DockMode::Left => {
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = (child_size.0, remaining_space.1);

                    offset.0 += child_size.0;
                    remaining_space.0 -= child_size.0;

                    (pos, size)
                },
                DockMode::Right => {
                    remaining_space.0 -= child_size.0;
                    let pos = (
                        node_position.0 + offset.0 + remaining_space.0,
                        node_position.1 + offset.1
                    );
                    let size = (child_size.0, remaining_space.1);

                    (pos, size)
                },
            };

            layout.position.x = child_pos.0;
            layout.position.y = child_pos.1;
            layout.size.x = child_final_size.0;
            layout.size.y = child_final_size.1;
        }
    }
    fn layout_stack(
        entities: &mut Vec<Entity>,
        layouts: &mut Vec<UiNodeLayout>,
        children_indices: &Vec<usize>,
        node_position: (f32, f32),
        node_size: (f32, f32),
        horizontal: bool,
        spacing: f32,
        padding: Padding,
        packing: PackingMode,
        alignment: Alignment,
        stack_direction: StackDirection,
    ) {
        let mut operable_children_indices = children_indices.iter().filter_map(|&child_index| {
            let child = &mut entities[child_index];
            let layout = &mut layouts[child.ui_layout.unwrap()];
            if let Some(relation) = &layout.parent_relation {
                match relation {
                    ParentRelation::Independent { .. } => None,
                    ParentRelation::Docking(_) => None,
                }
            } else {
                Some(child_index)
            }
        }).collect::<Vec<_>>();
        if operable_children_indices.is_empty() {
            return;
        }

        // inner space AFTER padding
        let inner_space = (
            node_size.0 - padding.left - padding.right,
            node_size.1 - padding.top - padding.bottom,
        );

        // calculate sizes + determine fill distribution
        let mut total_fill_weight = 0.0;
        let mut used_space = 0.0;
        let mut child_sizes = Vec::new();

        // apply reversals
        if matches!(packing, PackingMode::End) {
            operable_children_indices.reverse();
        }
        if matches!(stack_direction, StackDirection::Reverse) {
            operable_children_indices.reverse();
        }

        for &child_index in &operable_children_indices {
            let child = &mut entities[child_index];
            let layout = &mut layouts[child.ui_layout.unwrap()];

            let sizes = Self::calculate_size(&layout.width, &layout.height, &inner_space);
            let size = if horizontal { sizes.0 } else { sizes.1 };

            if size.1 {
                total_fill_weight += size.0;
                child_sizes.push(0.0); // replaced later
            } else {
                used_space += size.0;
                child_sizes.push(size.0);
            }
            /*
            let size_mode = if horizontal {
                &child.width
            } else {
                &child.height
            };

            match size_mode {
                Size::Absolute(s) => {
                    used_space += s;
                    child_sizes.push(*s);
                },
                Size::Factor(f) => {
                    let size = if horizontal { inner_space.0 * f } else { inner_space.1 * f };
                    used_space += size;
                    child_sizes.push(size);
                },
                Size::FillFactor(weight) => {
                    total_fill_weight += weight;
                    child_sizes.push(0.0); // replaced later
                },
                Size::Auto => {
                    let size = 100.0; // TODO: Calculate from content
                    used_space += size;
                    child_sizes.push(size);
                },
                Size::Copy => {
                    panic!("Cannot have both width and height copy eachother")
                }
            }
            */
        }

        // implement spacing
        if children_indices.len() > 1 {
            used_space += spacing * (children_indices.len() - 1) as f32;
        }

        // distribute remaining space to FillFactor children
        let primary_axis_space = if horizontal { inner_space.0 } else { inner_space.1 };
        let remaining_space = (primary_axis_space - used_space).max(0.0);

        for (idx, &child_index) in operable_children_indices.iter().enumerate() {
            let child = &mut entities[child_index];
            let layout = &mut layouts[child.ui_layout.unwrap()];
            let size_mode = if horizontal { &layout.width } else { &layout.height };

            if let Size::FillFactor(weight) = size_mode {
                child_sizes[idx] = if total_fill_weight > 0.0 {
                    remaining_space * (weight / total_fill_weight)
                } else {
                    0.0
                };
            }
        }

        // starting position based on packing
        let mut current_pos = match packing {
            PackingMode::Start => 0.0,
            PackingMode::End => primary_axis_space,
            PackingMode::Center => (primary_axis_space - used_space) * 0.5,
            PackingMode::SpaceIncludeEdge => {
                if children_indices.len() > 0 {
                    primary_axis_space / (children_indices.len() + 1) as f32
                } else {
                    0.0
                }
            },
            PackingMode::SpaceExcludeEdge => 0.0,
        };

        let item_spacing = match packing {
            PackingMode::SpaceIncludeEdge => {
                if children_indices.len() > 0 {
                    primary_axis_space / (children_indices.len() + 1) as f32
                } else {
                    0.0
                }
            },
            PackingMode::SpaceExcludeEdge => {
                if children_indices.len() > 1 {
                    (remaining_space + spacing * (children_indices.len() - 1) as f32) / (children_indices.len() - 1) as f32
                } else {
                    0.0
                }
            },
            _ => spacing,
        };

        // position children
        for (idx, &child_index) in operable_children_indices.iter().enumerate() {
            let child = &mut entities[child_index];
            let layout = &mut layouts[child.ui_layout.unwrap()];
            let primary_size = child_sizes[idx];

            // flip if end
            let actual_pos = if matches!(packing, PackingMode::End) {
                current_pos - primary_size
            } else {
                current_pos
            };

            // cross-axis size
            let cross_size = if horizontal {
                match layout.height {
                    Size::Absolute(h) => h,
                    Size::Factor(f) => inner_space.1 * f,
                    Size::FillFactor(_) => inner_space.1,
                    Size::Auto => 100.0,
                    Size::Copy => primary_size
                }
            } else {
                match layout.width {
                    Size::Absolute(w) => w,
                    Size::Factor(f) => inner_space.0 * f,
                    Size::FillFactor(_) => inner_space.0,
                    Size::Auto => 100.0,
                    Size::Copy => primary_size
                }
            };

            // cross-axis position based on alignment
            let cross_pos = match alignment {
                Alignment::Start => 0.0,
                Alignment::Center => (if horizontal { inner_space.1 } else { inner_space.0 } - cross_size) / 2.0,
                Alignment::End => if horizontal { inner_space.1 - cross_size } else { inner_space.0 - cross_size },
                Alignment::Stretch => 0.0,
            };

            let final_cross_size = match alignment {
                Alignment::Stretch => if horizontal { inner_space.1 } else { inner_space.0 },
                _ => cross_size,
            };

            let (child_pos, child_final_size) = if horizontal {
                (
                    (
                        node_position.0 + padding.left + actual_pos,
                        node_position.1 + padding.top + cross_pos
                    ),
                    (primary_size, final_cross_size)
                )
            } else {
                (
                    (
                        node_position.0 + padding.left + cross_pos,
                        node_position.1 + padding.top + actual_pos
                    ),
                    (final_cross_size, primary_size)
                )
            };

            layout.position.x = child_pos.0;
            layout.position.y = child_pos.1;
            layout.size.x = child_final_size.0;
            layout.size.y = child_final_size.1;

            // to next child start pos
            if matches!(packing, PackingMode::End) {
                current_pos -= primary_size + item_spacing;
            } else {
                current_pos += primary_size + item_spacing;
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GUIQuadSendable {
    pub additive_color: [f32; 4],

    pub multiplicative_color: [f32; 4],

    pub resolution: [i32; 2],

    pub clip_min: [f32; 2],
    pub clip_max: [f32; 2],

    pub position: [f32; 2],

    pub scale: [f32; 2],

    pub corner_radius: f32,

    pub image: i32,
}