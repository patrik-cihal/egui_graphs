#[cfg(feature = "events")]
use crate::events::{
    Event, PayloadNodeClick, PayloadNodeDeselect, PayloadNodeDoubleClick, PayloadNodeDragEnd,
    PayloadNodeDragStart, PayloadNodeMove, PayloadNodeSelect, PayloadPan, PyaloadZoom,
};
use crate::{
    computed::ComputedState,
    draw::{Drawer, FnEdgeDraw, FnNodeDraw},
    metadata::Metadata,
    settings::SettingsNavigation,
    settings::{SettingsInteraction, SettingsStyle},
    Graph, Node, default_node_draw, default_edges_draw,
};
#[cfg(feature = "events")]
use crossbeam::channel::Sender;
use egui::{Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use petgraph::{stable_graph::NodeIndex, EdgeType};

pub type FnNodeDetect<N> =
    fn(&Metadata, &Node<N>, Vec2, &SettingsStyle) -> bool;

/// Widget for visualizing and interacting with graphs.
///
/// It implements [egui::Widget] and can be used like any other widget.
///
/// The widget uses a mutable reference to the [petgraph::stable_graph::StableGraph<super::Node<N>, super::Edge<E>>]
/// struct to visualize and interact with the graph. `N` and `E` is arbitrary client data associated with nodes and edges.
/// You can customize the visualization and interaction behavior using [SettingsInteraction], [SettingsNavigation] and [SettingsStyle] structs.
///
/// When any interaction or node property change occurs, the widget sends [Change] struct to the provided
/// [Sender<Change>] channel, which can be set via the `with_interactions` method. The [Change] struct contains information about
/// a change that occurred in the graph. Client can use this information to modify external state of his application if needed.
///
/// When the user performs navigation actions (zoom & pan or fit to screen), they do not
/// produce changes. This is because these actions are performed on the global coordinates and do not change any
/// properties of the nodes or edges.
pub struct GraphView<'a, N: Clone, E: Clone, Ty: EdgeType> {
    settings_interaction: SettingsInteraction,
    settings_navigation: SettingsNavigation,
    settings_style: SettingsStyle,
    g: &'a mut Graph<N, E, Ty>,

    edge_draw_fn: FnEdgeDraw<N, E, Ty>,
    node_draw_fn: FnNodeDraw<N, E, Ty>,
    node_detect_fn: FnNodeDetect<N>,

    #[cfg(feature = "events")]
    events_publisher: Option<&'a Sender<Event>>,
}

impl<'a, N: Clone, E: Clone, Ty: EdgeType> Widget for &mut GraphView<'a, N, E, Ty> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (resp, p) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        let mut meta = Metadata::get(ui);
        let mut computed = self.compute_state();

        self.handle_fit_to_screen(&resp, &mut meta, &computed);
        self.handle_navigation(ui, &resp, &mut meta, &computed);

        self.handle_node_drag(&resp, &mut computed, &mut meta);
        self.handle_click(&resp, &mut meta, &computed);

        Drawer::new(
            p,
            self.g,
            &self.settings_style,
            &meta,
            self.node_draw_fn,
            self.edge_draw_fn,
        )
        .draw();

        meta.store_into_ui(ui);
        ui.ctx().request_repaint();

        resp
    }
}

impl<'a, N: Clone, E: Clone, Ty: EdgeType> GraphView<'a, N, E, Ty> {
    /// Creates a new `GraphView` widget with default navigation and interactions settings.
    /// To customize navigation and interactions use `with_interactions` and `with_navigations` methods.
    pub fn new(g: &'a mut Graph<N, E, Ty>) -> Self {
        Self {
            g,

            settings_style: Default::default(),
            settings_interaction: Default::default(),
            settings_navigation: Default::default(),

            node_draw_fn: default_node_draw,
            edge_draw_fn: default_edges_draw,
            node_detect_fn: Self::node_detect,

            #[cfg(feature = "events")]
            events_publisher: Default::default(),
        }
    }

    /// Sets a function that will be called instead of the default drawer for every node to draw custom shapes.
    pub fn with_custom_node_draw(mut self, func: FnNodeDraw<N, E, Ty>) -> Self {
        self.node_draw_fn = func;
        self
    }

    /// Sets a function that will be called instead of the default drawer for every pair of nodes connected with edges to draw custom shapes.
    pub fn with_custom_edge_draw(mut self, func: FnEdgeDraw<N, E, Ty>) -> Self {
        self.edge_draw_fn = func;
        self
    }

    pub fn with_custom_node_detect(mut self, func: FnNodeDetect<N>) -> Self {
        self.node_detect_fn = func;
        self
    }

    /// Makes widget interactive according to the provided settings.
    pub fn with_interactions(mut self, settings_interaction: &SettingsInteraction) -> Self {
        self.settings_interaction = settings_interaction.clone();
        self
    }

    /// Modifies default behaviour of navigation settings.
    pub fn with_navigations(mut self, settings_navigation: &SettingsNavigation) -> Self {
        self.settings_navigation = settings_navigation.clone();
        self
    }

    /// Modifies default style settings.
    pub fn with_styles(mut self, settings_style: &SettingsStyle) -> Self {
        self.settings_style = settings_style.clone();
        self
    }

    /// Resets navigation metadata
    pub fn reset_metadata(ui: &mut Ui) {
        Metadata::default().store_into_ui(ui);
    }

    #[cfg(feature = "events")]
    pub fn with_events(mut self, events_publisher: &'a Sender<Event>) -> Self {
        self.events_publisher = Some(events_publisher);
        self
    }

    fn compute_state(&mut self) -> ComputedState {
        let mut computed = ComputedState::default();

        let n_idxs = self.g.g.node_indices().collect::<Vec<_>>();
        n_idxs.iter().for_each(|idx| {
            let comp = computed.compute_for_node(self.g, *idx);

            let n = self.g.node_mut(*idx).unwrap();
            n.set_computed(comp);

            computed.comp_iter_bounds(n, &self.settings_style);
        });

        computed
    }

    /// Fits the graph to the screen if it is the first frame or
    /// fit to screen setting is enabled;
    fn handle_fit_to_screen(&self, r: &Response, meta: &mut Metadata, comp: &ComputedState) {
        if !meta.first_frame && !self.settings_navigation.fit_to_screen_enabled {
            return;
        }

        self.fit_to_screen(&r.rect, meta, comp);
        meta.first_frame = false;
    }

    fn handle_click(&mut self, resp: &Response, meta: &mut Metadata, comp: &ComputedState) {
        if !resp.clicked() && !resp.double_clicked() {
            return;
        }

        let clickable = self.settings_interaction.clicking_enabled
            || self.settings_interaction.selection_enabled
            || self.settings_interaction.selection_multi_enabled;

        if !(clickable) {
            return;
        }

        let node = self
            .node_by_screen_pos(meta, resp.hover_pos().unwrap());

        if node.is_none() {
            // click on empty space
            let selectable = self.settings_interaction.selection_enabled
                || self.settings_interaction.selection_multi_enabled;
            if selectable {
                self.deselect_all(comp);
            }
            return;
        }

        // first click of double click is handleed by the lib as single click
        // so if you double click a node it will handle it as single click at first
        // and only after as double click
        let node_idx = node.unwrap().0;
        if resp.double_clicked() {
            self.handle_node_double_click(node_idx);
            return;
        }
        self.handle_node_click(node_idx, comp);
    }

    fn handle_node_double_click(&mut self, idx: NodeIndex) {
        if !self.settings_interaction.clicking_enabled {
            return;
        }

        if self.settings_interaction.clicking_enabled {
            self.set_node_double_clicked(idx);
        }
    }

    fn handle_node_click(&mut self, idx: NodeIndex, comp: &ComputedState) {
        if !self.settings_interaction.clicking_enabled
            && !self.settings_interaction.selection_enabled
        {
            return;
        }

        if self.settings_interaction.clicking_enabled {
            self.set_node_clicked(idx);
        }

        if !self.settings_interaction.selection_enabled {
            return;
        }

        let n = self.g.node(idx).unwrap();
        if n.selected() {
            self.deselect_node(idx);
            return;
        }

        if !self.settings_interaction.selection_multi_enabled {
            self.deselect_all(comp);
        }

        self.select_node(idx);
    }

    fn handle_node_drag(&mut self, resp: &Response, comp: &mut ComputedState, meta: &mut Metadata) {
        if !self.settings_interaction.dragging_enabled {
            return;
        }

        if resp.drag_started() {
            if let Some((idx, _)) =
                self
                    .node_by_screen_pos(meta, resp.hover_pos().unwrap())
            {
                self.set_drag_start(idx);
            }
        }

        if resp.dragged()
            && comp.dragged.is_some()
            && (resp.drag_delta().x.abs() > 0. || resp.drag_delta().y.abs() > 0.)
        {
            let n_idx_dragged = comp.dragged.unwrap();
            let delta_in_graph_coords = resp.drag_delta() / meta.zoom;
            self.move_node(n_idx_dragged, delta_in_graph_coords);
        }

        if resp.drag_released() && comp.dragged.is_some() {
            let n_idx = comp.dragged.unwrap();
            self.set_drag_end(n_idx);
        }
    }

    /// Finds node by position. Can be optimized by using a spatial index like quad-tree if needed.
    pub fn node_by_screen_pos(
        &self,
        meta: &'a Metadata,
        screen_pos: Pos2,
    ) -> Option<(NodeIndex, &Node<N>)> {
        let pos_in_graph = (screen_pos.to_vec2() - meta.pan) / meta.zoom;
        self.g.nodes_iter().find(|(_, n)| {
            (self.node_detect_fn)(meta, n, pos_in_graph, &self.settings_style)
        })
    }

    fn fit_to_screen(&self, rect: &Rect, meta: &mut Metadata, comp: &ComputedState) {
        // calculate graph dimensions with decorative padding
        let bounds = comp.graph_bounds();
        let mut diag = bounds.max - bounds.min;

        // if the graph is empty or consists from one node, use a default size
        if diag == Vec2::ZERO {
            diag = Vec2::new(1., 100.);
        }

        let graph_size = diag * (1. + self.settings_navigation.screen_padding);
        let (width, height) = (graph_size.x, graph_size.y);

        // calculate canvas dimensions
        let canvas_size = rect.size();
        let (canvas_width, canvas_height) = (canvas_size.x, canvas_size.y);

        // calculate zoom factors for x and y to fit the graph inside the canvas
        let zoom_x = canvas_width / width;
        let zoom_y = canvas_height / height;

        // choose the minimum of the two zoom factors to avoid distortion
        let new_zoom = zoom_x.min(zoom_y);

        // calculate the zoom delta and call handle_zoom to adjust the zoom factor
        let zoom_delta = new_zoom / meta.zoom - 1.0;
        self.zoom(rect, zoom_delta, None, meta);

        // calculate the center of the graph and the canvas
        let graph_center = (bounds.min.to_vec2() + bounds.max.to_vec2()) / 2.0;

        // adjust the pan value to align the centers of the graph and the canvas
        let new_pan = rect.center().to_vec2() - graph_center * new_zoom;
        self.set_pan(new_pan, meta);
    }

    fn handle_navigation(
        &self,
        ui: &Ui,
        resp: &Response,
        meta: &mut Metadata,
        comp: &ComputedState,
    ) {
        self.handle_zoom(ui, resp, meta);
        self.handle_pan(resp, meta, comp);
    }

    fn handle_zoom(&self, ui: &Ui, resp: &Response, meta: &mut Metadata) {
        if !self.settings_navigation.zoom_and_pan_enabled {
            return;
        }

        ui.input(|i| {
            let delta = i.zoom_delta();
            if delta == 1. {
                return;
            }

            let step = self.settings_navigation.zoom_speed * (1. - delta).signum();
            self.zoom(&resp.rect, step, i.pointer.hover_pos(), meta);
        });
    }

    fn handle_pan(&self, resp: &Response, meta: &mut Metadata, comp: &ComputedState) {
        if !self.settings_navigation.zoom_and_pan_enabled {
            return;
        }

        if resp.dragged()
            && comp.dragged.is_none()
            && (resp.drag_delta().x.abs() > 0. || resp.drag_delta().y.abs() > 0.)
        {
            let new_pan = meta.pan + resp.drag_delta();
            self.set_pan(new_pan, meta);
        }
    }

    /// Zooms the graph by the given delta. It also compensates with pan to keep the zoom center in the same place.
    fn zoom(&self, rect: &Rect, delta: f32, zoom_center: Option<Pos2>, meta: &mut Metadata) {
        let center_pos = match zoom_center {
            Some(center_pos) => center_pos - rect.min,
            None => rect.center() - rect.min,
        };
        let graph_center_pos = (center_pos - meta.pan) / meta.zoom;
        let factor = 1. + delta;
        let new_zoom = meta.zoom * factor;

        let pan_delta = graph_center_pos * meta.zoom - graph_center_pos * new_zoom;
        let new_pan = meta.pan + pan_delta;

        self.set_pan(new_pan, meta);
        self.set_zoom(new_zoom, meta);
    }

    fn select_node(&mut self, idx: NodeIndex) {
        let n = self.g.node_mut(idx).unwrap();
        n.set_selected(true);

        #[cfg(feature = "events")]
        self.publish_event(Event::NodeSelect(PayloadNodeSelect { id: idx.index() }));
    }

    fn deselect_node(&mut self, idx: NodeIndex) {
        let n = self.g.node_mut(idx).unwrap();
        n.set_selected(false);

        #[cfg(feature = "events")]
        self.publish_event(Event::NodeDeselect(PayloadNodeDeselect { id: idx.index() }));
    }

    fn set_node_clicked(&mut self, idx: NodeIndex) {
        #[cfg(feature = "events")]
        self.publish_event(Event::NodeClick(PayloadNodeClick { id: idx.index() }));
    }

    fn set_node_double_clicked(&mut self, idx: NodeIndex) {
        #[cfg(feature = "events")]
        self.publish_event(Event::NodeDoubleClick(PayloadNodeDoubleClick {
            id: idx.index(),
        }));
    }

    fn deselect_all(&mut self, comp: &ComputedState) {
        comp.selected.iter().for_each(|idx| {
            self.deselect_node(*idx);
        });
    }

    fn move_node(&mut self, idx: NodeIndex, delta: Vec2) {
        let n = self.g.node_mut(idx).unwrap();
        n.set_location(n.location() + delta);

        #[cfg(feature = "events")]
        self.publish_event(Event::NodeMove(PayloadNodeMove {
            id: idx.index(),
            diff: delta.into(),
        }));
    }

    fn set_drag_start(&mut self, idx: NodeIndex) {
        let n = self.g.node_mut(idx).unwrap();
        n.set_dragged(true);

        #[cfg(feature = "events")]
        self.publish_event(Event::NodeDragStart(PayloadNodeDragStart {
            id: idx.index(),
        }));
    }

    fn set_drag_end(&mut self, idx: NodeIndex) {
        let n = self.g.node_mut(idx).unwrap();
        n.set_dragged(false);

        #[cfg(feature = "events")]
        self.publish_event(Event::NodeDragEnd(PayloadNodeDragEnd { id: idx.index() }));
    }

    fn set_pan(&self, val: Vec2, meta: &mut Metadata) {
        let diff = val - meta.pan;
        meta.pan = val;

        #[cfg(feature = "events")]
        self.publish_event(Event::Pan(PayloadPan { diff: diff.into() }));
    }

    fn set_zoom(&self, val: f32, meta: &mut Metadata) {
        let diff = val - meta.zoom;
        meta.zoom = val;

        #[cfg(feature = "events")]
        self.publish_event(Event::Zoom(PyaloadZoom { diff }));
    }

    #[cfg(feature = "events")]
    fn publish_event(&self, event: Event) {
        if let Some(sender) = self.events_publisher {
            sender.send(event).unwrap();
        }
    }
    fn node_detect(meta: &Metadata, n: &Node<N>, pos_in_graph: Vec2, settings_style: &SettingsStyle) -> bool {
        let dist_to_node = (n.location() - pos_in_graph).length();
        dist_to_node <= n.screen_radius(meta, &settings_style) / meta.zoom
    }
}
