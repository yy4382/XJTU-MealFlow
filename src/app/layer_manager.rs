use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{
    actions::{Action, LayerManageAction, Layers},
    libs::fetcher::MockMealFetcher,
    page::{
        Layer, analysis::Analysis, cookie_input::CookieInput, fetch::Fetch, help_popup::HelpPopup,
        home::Home, transactions::Transactions,
    },
    tui::Event,
};
use color_eyre::Result;
use ratatui::Frame;
use tracing::{info, warn};

use super::RootState;

pub(super) struct BoxedLayer(Box<dyn Layer>);
impl Deref for BoxedLayer {
    type Target = dyn Layer;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl DerefMut for BoxedLayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}
impl From<Box<dyn Layer>> for BoxedLayer {
    fn from(layer: Box<dyn Layer>) -> Self {
        Self(layer)
    }
}
impl BoxedLayer {
    fn into_layer_config(self, render: bool) -> LayerConfig {
        LayerConfig {
            layer: self,
            render,
        }
    }
}

pub(super) struct LayerConfig {
    layer: BoxedLayer,
    render: bool,
}

impl Deref for LayerConfig {
    type Target = BoxedLayer;

    fn deref(&self) -> &Self::Target {
        &self.layer
    }
}
impl DerefMut for LayerConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.layer
    }
}

pub(super) struct LayerManager {
    layers: Vec<LayerConfig>,
}

impl Deref for LayerManager {
    type Target = Vec<LayerConfig>;

    fn deref(&self) -> &Self::Target {
        &self.layers
    }
}
impl DerefMut for LayerManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.layers
    }
}

impl LayerManager {
    pub(super) fn new(layer: Box<dyn Layer>) -> Self {
        Self {
            layers: vec![LayerConfig {
                layer: BoxedLayer(layer),
                render: true,
            }],
        }
    }

    pub(super) fn render(&mut self, f: &mut Frame) {
        self.layers
            .iter_mut()
            .filter(|page| page.render)
            .for_each(|page| page.render(f, f.area()));
    }

    pub(super) fn handle_event(&self, event: Event) -> Result<()> {
        // TODO currently, we only handle events for the top layer. Is it necessary to handle events for all layers?
        let last_page = self.layers.last().expect("No page in stack");
        last_page.handle_events(event)
    }

    /// Handle LayerManageAction for root app, updating the layer stack
    pub(super) fn handle_layer_action(&mut self, action: &LayerManageAction, state: &RootState) {
        match action {
            LayerManageAction::Swap(target) => {
                self.layers.pop();
                self.layers.push(
                    LayerManager::get_layer(target, state)
                        .expect("Failed to get layer")
                        .into_layer_config(true),
                );
                info!(
                    "Swapping page to {}, current layer stack length {}",
                    target,
                    self.layers.len()
                );
            }
            LayerManageAction::Push(target) => {
                self.layers.last_mut().unwrap().render = target.render_self;
                self.layers.push(
                    LayerManager::get_layer(&target.layer, state)
                        .expect("Failed to get layer")
                        .into_layer_config(true),
                );
                info!(
                    "Pushing a {} page, current page will {} render, new layer stack length {}",
                    target.layer,
                    if target.render_self { "still" } else { "not" },
                    self.layers.len()
                );
            }
            LayerManageAction::Pop => {
                self.layers.pop();
                if self.layers.is_empty() {
                    self.layers.push(
                        LayerManager::get_layer(&Layers::Home, state)
                            .expect("Failed to get layer")
                            .into_layer_config(true),
                    );
                }
                self.layers.last_mut().unwrap().render = true;
                info!(
                    "Popping page, current layer stack length {}",
                    self.layers.len()
                );
            }
        }
    }

    /// Passing the action to the layer who is interested in it (currently only the top layer)
    pub(super) fn handle_action(&mut self, action: Action) {
        // TODO currently we only handle actions for the top layer. Is it necessary to handle actions for all layers?
        // Maybe we should add a `interested_actions` pattern to LayerConfig
        // to filter out actions that are not interested in and let the layer handle it
        self.layers
            .last_mut()
            .expect("No page in stack")
            .update(action);
    }

    /// Get a new layer based on the given layer type
    fn get_layer(layer: &Layers, state: &RootState) -> Option<BoxedLayer> {
        let mut page = match layer.clone() {
            Layers::Home => Box::new(Home {
                tx: state.action_tx.clone().into(),
            }) as Box<dyn Layer>,
            Layers::Transaction(filter_opt) => Box::new(Transactions::new(
                filter_opt,
                state.action_tx.clone().into(),
                state.manager.clone(),
            )),
            Layers::Fetch => Box::new(
                Fetch::new(
                    state.action_tx.clone().into(),
                    state.manager.clone(),
                    state.input_mode,
                )
                .client(if state.config.fetch.use_mock_data {
                    MockMealFetcher::default()
                        .set_sim_delay(Duration::from_secs(1))
                        .per_page(50)
                } else {
                    Default::default()
                }),
            ),
            Layers::CookieInput => Box::new(CookieInput::new(
                state.action_tx.clone().into(),
                state.manager.clone(),
                state.input_mode,
            )),
            Layers::Help(help_msg) => {
                let help = HelpPopup::new(state.action_tx.clone().into(), help_msg.clone());
                match help {
                    Some(help) => Box::new(help) as Box<dyn Layer>,
                    None => {
                        warn!("Help message is empty");
                        return None;
                    }
                }
            }
            Layers::Analysis => Box::new(Analysis::new(
                state.action_tx.clone().into(),
                state.manager.clone(),
            )),
        };
        page.init();
        Some(page.into())
    }
}
