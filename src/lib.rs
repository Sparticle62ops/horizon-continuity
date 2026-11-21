//! 1:1 Horizon Rust port of Continuity by PepperCode1 (LGPL-3.0-only,
//! https://github.com/PepperCode1/Continuity).
//!
//! The 1:1 logic core lives in the sibling modules, untouched: the CTM
//! properties parsing (`properties`, `properties_loader`,
//! `connecting_properties`, `overlay_properties`, `parsing`), the quad
//! processors (`processor_base`, `processor_compact`, `processor_enums`,
//! `quad_processors`, `quad_util`, `overlay_processors`,
//! `processing_context_impl`), the sprite providers (`simple_sprite_provider`,
//! `random_index_provider`), the model layers (`model`, `model_wrapping`,
//! `baked_model_manager_reload`, `emissive_baked_model`,
//! `emissive_suffix_loader`, `render_api`, `resource_redirect`, `math_util`),
//! the custom block layers, the config and its screen, and the engine seams
//! (`seams.rs`) where the Java mod calls into Minecraft.
//!
//! The mod API wires what the pinned SDK (api 0.2, f00125f61f) reaches:
//! - the original's four config booleans as settings, so Horizon's
//!   Options > Mods page holds them, same defaults;
//! - `client_setup` initializes the core exactly as
//!   ContinuityClient.onInitializeClient does (the CTM loader registry,
//!   the model-wrapping plugin, the custom-block-layers reload listener),
//!   with the wrap flags applied from the settings;
//! - `client_resources_reloaded` re-applies the wrap flags on every
//!   successful resource reload, the original's
//!   ValueFlag::ReloadWorldRenderer effect.
//!
//! Gaps, tracked: the quad wrap itself runs in the engine's model-bake
//! pass, and the pinned mod API has no baked-geometry hook
//! (horizon-rust/horizon#861: `model_baking_completed` gives no view of
//! the baked geometry and edits nothing), so models bake un-wrapped until
//! that lands. The reload event also carries no ResourceManager handle,
//! so the core's reload listeners (the custom-block-layers scan, the
//! .ctm.properties redirect) wait on the same engine integration.
use std::rc::Rc;

use mod_api::{
    self, ClientHost, ClientResourcesReloadedEvent, ClientSetupEvent, RegistrationContext,
    Registrar, SettingSpec,
};

pub mod baked_model_manager_reload;
pub mod config;
pub mod config_screen;
pub mod connecting_properties;
pub mod continuity_client;
pub mod custom_block_layers;
pub mod emissive_baked_model;
pub mod emissive_suffix_loader;
pub mod math_util;
pub mod model;
pub mod model_wrapping;
pub mod overlay_processors;
pub mod overlay_properties;
pub mod parsing;
pub mod processing_context_impl;
pub mod processor_base;
pub mod processor_compact;
pub mod processor_enums;
pub mod properties;
pub mod properties_loader;
pub mod quad_processors;
pub mod quad_util;
pub mod random_index_provider;
pub mod render_api;
pub mod resource_redirect;
pub mod seams;
pub mod simple_sprite_provider;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// The original's four config booleans as settings, same defaults.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let cfg = config::config_instance();
        for (key, label) in [
            ("connected_textures", "setting.continuity.connected_textures"),
            ("emissive_textures", "setting.continuity.emissive_textures"),
            ("custom_block_layers", "setting.continuity.custom_block_layers"),
            ("use_manual_culling", "setting.continuity.use_manual_culling"),
        ] {
            let value = cfg.get_option(key).map(|o| o.get()).unwrap_or(true);
            let _ = registrar.register_setting(SettingSpec::toggle(key, label, value));
        }
    }

    /// 1:1 of ContinuityClient.onInitializeClient (the CTM loader registry)
    /// plus the model-wrapping and custom-block-layers core init, with the
    /// wrap flags applied from the settings the client has.
    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        continuity_client::on_initialize_client();
        model_wrapping::init();
        custom_block_layers::ReloadListener::init();
        sync_wrapping(host);
    }

    /// The settings' reload effect, as the original's config screen has it:
    /// the wrapping flags re-apply on every successful resource reload.
    fn client_resources_reloaded(
        &self,
        host: &mut dyn ClientHost,
        _event: &mut ClientResourcesReloadedEvent,
    ) {
        sync_wrapping(host);
    }
}

/// The wrap flags from the settings, driving the 1:1 wrapping handler the
/// engine-side plugin consults.
fn sync_wrapping(host: &dyn ClientHost) {
    let wrap_ctm = host.setting_bool("connected_textures").unwrap_or(true);
    let wrap_emissive = host.setting_bool("emissive_textures").unwrap_or(true);
    let handler = model_wrapping::ModelWrappingHandler::create(wrap_ctm, wrap_emissive);
    model_wrapping::continuity_set_model_wrapping_handler(handler.map(Rc::new));
}
