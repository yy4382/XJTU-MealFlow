use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{
    actions::{LayerManageAction, Layers},
    libs::fetcher::{MealFetcher, MockMealFetcher, RealMealFetcher},
    page::{
        Layer, analysis::Analysis, cookie_input::CookieInput, fetch::Fetch, help_popup::HelpPopup,
        home::Home, transactions::Transactions,
    },
    tui::Event,
};
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

#[derive(Debug, Clone, Default)]
pub enum EventHandlingStatus {
    /// The layer has handled the event the manager should not send it to the underlying layers
    Consumed,
    /// The layer has not handled the event and the manager should send it to the underlying layers
    #[default]
    ShouldPropagate,
}

impl EventHandlingStatus {
    pub fn consumed(&mut self) {
        *self = EventHandlingStatus::Consumed;
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

    pub(super) fn handle_event(&mut self, event: Event) {
        for layer in self.layers.iter_mut().rev() {
            let status = layer.handle_events(&event);
            if matches!(status, EventHandlingStatus::Consumed) {
                break;
            }
        }
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
                Fetch::new(state.action_tx.clone().into(), state.manager.clone()).client(
                    if state.config.fetch.use_mock_data {
                        MealFetcher::Mock(
                            MockMealFetcher::default()
                                .set_sim_delay(Duration::from_secs(1))
                                .per_page(50),
                        )
                    } else {
                        MealFetcher::Real(RealMealFetcher::default())
                    },
                ),
            ),
            Layers::CookieInput => Box::new(CookieInput::new(
                state.action_tx.clone().into(),
                state.manager.clone(),
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
