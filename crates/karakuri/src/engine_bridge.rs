//! Engine coordination, presentation sinks, and view bridging.

use std::sync::Arc;

use karakuri_console::focus::{self, Step};
use karakuri_console::panel::Panel;
use karakuri_console::view::{
    self, picture_rect, preview_rects, Basis, Budgeted, Go, Picture, Reading, RowKind, Scope,
    TransitionSettings, View, DECKS, DECK_LETTERS,
};
use karakuri_console::{egui, egui_wgpu};
use karakuri_engine::estimate::Estimate;
use karakuri_engine::governor::{Basis as Spent, Report};
use karakuri_engine::set::{Layering, Published};
use karakuri_engine::transition::Selection;
use karakuri_engine::transport::Sync as EngineSync;
use karakuri_engine::{
    Blend, Control, Cut, Deck, DeckSlot as EngineSlot, Event, Gpu, HotSwap, Look, Mask, MaskKind,
    Present, Residency, Set, Sink, Skip, TonemapOp,
};
use karakuri_environment::{audio, history, mix, setfile, watch, Asked};
use karakuri_ir::Kind as Layer;
use karakuri_layout::Layout;
use karakuri_mcp as mcp;
use karakuri_operation::{BlendMode, Operation, SetTransfer};
use karakuri_operation_record::{Current, Written};
use karakuri_store::record::{DeckSlot, Record};
use karakuri_store::store::Store;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use crate::{
    deck_letter, ms, rewired, watched, Costs, Keeping, Sources, ASKED_TO_PRIME, CANVAS, ON_AIR,
    SEED_SALT, SLOTS,
};

/// **The look this window opens under**, and it is where [`Engine::look`]
/// starts rather than what every frame is drawn under.
///
/// [`compose`] writes the tone-map uniform on every frame from the
/// [`Committed`] the closure hands back, so a harness with nothing to say
/// about the look still has to say something. **This file now has something to
/// say**: the transport row's two look controls move [`Engine::look`] through
/// a record, so what a frame is committed under is that field and this is only
/// its first value.
///
/// **Aces, and it is still not this program inventing an aesthetic.** ADR-0037
/// picked the default *by looking* and left the trade open — *"ACES works on
/// stage … AgX is kind to material"* — and recorded that the choice is only
/// about what happens when nobody chooses, *"and can be changed on the
/// night"*. Until this pass nobody could change it here; now a press can, and
/// the constant is what the night starts at.
pub(crate) const LOOK: Look = Look {
    op: TonemapOp::Aces,
    exposure: 1.0,
    white_point: 1.0,
};

/// What the picture and every preview are rendered in: an sRGB format, so the
/// hardware does the one encode `Present`'s shader relies on ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
pub(crate) const PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// **The same texels, read as if they were not sRGB, which is how `egui` wants
/// them.**
///
/// `egui`'s fragment shader says so outright — *"We expect 'normal' textures
/// that are NOT sRGB-aware"* — and multiplies the sample by the vertex tint in
/// gamma space. A view in [`PICTURE_FORMAT`] would have the hardware decode to
/// linear on the way in and `egui` would then write those linear values into a
/// gamma-space surface, which is a picture that comes out visibly dark with no
/// error anywhere. So the render target is sRGB, the sampled view is this, and
/// the encode still happens exactly once — in the present pass.
pub(crate) const PICTURE_SAMPLED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// What the view `egui` samples is called on the device. The **texture**
/// carries the name that says which of the two it is — see
/// [`Presented::label`] — and this says which view of that texture it is, so
/// the pair reads as one thing in a validation message rather than as two with
/// the same name.
pub(crate) const SAMPLED_LABEL: &str = "as egui reads it";

/// **A texture the engine presents into and the panel samples**: the texture
/// itself, the view [`Present::draw`] draws into, the registration `egui`
/// reads it by, and its size in physical pixels.
///
/// **Two call sites on the day it is written**, which is this repository's
/// rule about an abstraction and is the same standing
/// [`karakuri_console::view::bay_head`] has: the Program bay's picture, and
/// deck A's preview cell under it. The four fields were `Engine`'s own until
/// the second one needed them, and every argument written on them then is
/// written on them here, because all of it is still true of both.
///
/// # It is a [`Sink`], and the aiming is the half a `Sink` cannot carry
///
/// [`Sink::acquire`] is handed a `&Gpu` and nothing else, and sizing one of
/// these needs the region's rectangle, the scale factor and the
/// [`egui_wgpu::Renderer`] the registration lives in. So the frame does that
/// first, through [`Presented::aim`], and `acquire` answers from what it was
/// aimed at: a rectangle means a target, no rectangle means [`Skip::Transient`]
/// and nothing at all is drawn into it.
///
/// **The split is where it is because of what a test can call.** Deciding
/// *which rectangle, at what size* inside `window_event` is deciding it
/// somewhere `winit` will not let a test reach — and sizing deck A's texture
/// from the picture's rectangle was injected there once and every test still
/// passed. [`aims`] and [`Presented::aim`] are that decision, whole, outside
/// the event handler; `mod gpu` calls them the way the frame does.
///
/// **Neither of these is presented anywhere**, which is why
/// [`Sink::present`] is `Ok(())` for both: the picture and the preview cell
/// are textures the panel samples, and the thing that reaches a display is the
/// window — which is not a sink here. `karakuri-cli`'s window shows the
/// canvas; this window shows the panel, and the panel goes in `finally`.
pub(crate) struct Presented {
    /// The texture. Held because the views below are of it, and because
    /// freeing the `egui` registration does not free this — `egui-wgpu` stores
    /// a bind group for a registered native texture and no texture at all, so
    /// dropping this is the only thing that releases the memory. Read only by
    /// `mod gpu`, which asks it what size it came out, and that is the whole
    /// of why the lint has to be told: the window does not read it and the
    /// window is not what it is for.
    #[allow(dead_code)]
    pub(crate) texture: wgpu::Texture,
    /// What [`Present::draw`] draws into: sRGB, so the encode is the
    /// hardware's.
    pub(crate) target: wgpu::TextureView,
    /// The registration `egui` draws by, of a view in
    /// [`PICTURE_SAMPLED_FORMAT`].
    pub(crate) id: egui::TextureId,
    /// The texture's size in physical pixels — **its region's**, not the
    /// window's.
    pub(crate) size: (u32, u32),
    /// What the texture is called on the device. Kept rather than passed to
    /// [`Presented::fit`], so the texture a resize makes is called what the
    /// one it replaces was called; and its own per texture, so a device
    /// message about the preview does not read as one about the picture.
    pub(crate) label: &'static str,
    /// **Whether this sink has a rectangle on screen this frame**, written by
    /// [`Presented::aim`] and read by nothing but [`Sink::acquire`].
    ///
    /// It is a field rather than an argument because the two questions are
    /// asked at different moments and by different callers: the frame aims
    /// every sink before it composes, and `compose` then asks each one for
    /// itself. A `Presented` that has never been aimed answers no, which is
    /// the right answer — it has a 1x1 placeholder texture and nothing has
    /// said where it goes.
    pub(crate) aimed: bool,
}

impl Presented {
    /// **A sink aimed at the rectangle its region has**, or at nothing where
    /// the region is folded away.
    ///
    /// It goes through [`Presented::aim`] rather than sizing the texture here,
    /// so that *which rectangle, at what size* has exactly one derivation in
    /// this file and the window's first frame cannot disagree with the window
    /// it opened at. The 1x1 below is never drawn into and never on screen: it
    /// is what `aim` replaces on the same statement.
    ///
    /// The `freed` it counts into is discarded, and that is the one place in
    /// this file where it may be. [`Engine::freed`] is a tally of registrations
    /// leaked by a **resize**, and this is construction — a caller that saw a
    /// 1 here would be told a leak had already happened before the window drew.
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        at: Option<egui::Rect>,
        scale: f32,
    ) -> Presented {
        let mut presented = Presented::made(gpu, renderer, label, (1, 1));
        let mut construction = 0;
        presented.aim(gpu, renderer, at, scale, &mut construction);
        presented
    }

    /// The texture, the view the engine draws into, and the registration
    /// `egui` reads it by, at a size in physical pixels. One constructor
    /// because the three are made together and are replaced together, and
    /// private to this type because a caller aims at a rectangle — turning one
    /// into a size is [`Presented::aim`]'s and [`Presented::fit`]'s alone.
    pub(crate) fn made(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        size: (u32, u32),
    ) -> Presented {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PICTURE_FORMAT,
            // COPY_SRC is not for the frame path — nothing here ever copies
            // one of these — it is what lets a test read a cell back and say
            // that a pass really was recorded into it. `Deck::slot_target`
            // carries the same flag for the same reason and says so.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            // Declared so the sampled view below may reinterpret it — the two
            // formats differ only in whether the transfer function is applied.
            view_formats: &[PICTURE_SAMPLED_FORMAT],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(SAMPLED_LABEL),
            format: Some(PICTURE_SAMPLED_FORMAT),
            ..Default::default()
        });
        let id = renderer.register_native_texture(&gpu.device, &sampled, wgpu::FilterMode::Linear);
        Presented {
            texture,
            target,
            id,
            size,
            label,
            aimed: false,
        }
    }

    /// **Where this sink goes this frame, and how big its texture therefore
    /// is** — one statement, and it is the whole of what [`Sink::acquire`]
    /// then answers from.
    ///
    /// Returns what the console should draw, or `None` where there is no
    /// rectangle. **The returned [`Picture`] carries the same rectangle the
    /// texture was just sized from and the id the sizing may have just
    /// replaced**, which is the pairing `karakuri_console::view::Picture`'s own
    /// documentation asks for: whoever sized the texture and whoever placed it
    /// are one statement, so a texture sized from the window and drawn into the
    /// picture's region cannot be written by accident, and a resize cannot
    /// leave a freed id in the view.
    ///
    /// **Called before [`compose`], and it has to be**: `acquire` takes only a
    /// `&Gpu`, and this needs the rectangle, the scale factor and the
    /// renderer. It is also a reallocation on the frames a size changed, so it
    /// belongs at the top of the frame rather than mid-pass — see
    /// [`Presented::fit`].
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        at: Option<egui::Rect>,
        scale: f32,
        freed: &mut usize,
    ) -> Option<Picture> {
        self.aimed = at.is_some();
        // **No rectangle, so nothing to fit.** The texture is left exactly as
        // it was rather than shrunk: a folded region is one an operator
        // unfolds, and remaking it small and large again would put two
        // reallocations on a fold that costs none. Nothing is drawn into it
        // meanwhile — `acquire` refuses, which is the whole of what a fold
        // has to mean.
        let rect = at?;
        self.fit(gpu, renderer, physical(rect, scale), freed);
        Some(Picture { id: self.id, rect })
    }

    /// **The region is a different size, so the texture is remade at that size
    /// and the registration it replaces is freed.**
    ///
    /// Returns whether anything was remade, which is `false` on all but a
    /// handful of frames — every frame of a drag on the program's height is
    /// one of them, and that is exactly the case the free is for. A
    /// `register_native_texture` per dragged frame with no `free_texture`
    /// beside it is a bind group and a sampler leaked per frame, for as long
    /// as somebody keeps hold of a divider.
    ///
    /// A reallocation, so it is the frame's first act and not something done
    /// mid-pass ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
    /// is about the render thread, and this is that thread; what it costs is
    /// paid on the frames a size changed and on no others).
    ///
    /// **`freed` is handed in rather than kept here** because the tally is the
    /// whole engine's — one number over both textures, which is what
    /// [`Engine::freed`] is and what `mod gpu` asserts on. Taking it as an
    /// argument is what makes it impossible for a call site to remake a
    /// texture and forget to count what it freed.
    pub(crate) fn fit(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        size: (u32, u32),
        freed: &mut usize,
    ) -> bool {
        if size == self.size {
            return false;
        }
        // Freed **before** the replacement is registered, which is the order
        // the leak is about rather than a tidiness: the atlas holds the old
        // bind group until this call and nothing else ever drops it.
        renderer.free_texture(&self.id);
        *freed += 1;
        // Carried across, because a resize is not an un-aiming: the rectangle
        // this was aimed at is the one it was just resized to.
        let aimed = self.aimed;
        *self = Presented::made(gpu, renderer, self.label, size);
        self.aimed = aimed;
        true
    }
}

/// **The two textures are sinks, and this is the whole of what that costs.**
///
/// `acquire` answers from [`Presented::aim`] and nothing else; `present` is
/// `Ok(())` because neither of these is presented anywhere — they are sampled
/// by the panel, and the panel is drawn in [`compose`]'s `finally` rather than
/// by a sink. See the type's own documentation.
impl Sink for Presented {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        match self.aimed {
            true => Ok(()),
            // **[`Skip::Transient`] and never a `Fault`.** A folded region is
            // an operator's choice and the next `f` over it undoes it, so
            // there is nothing to say about it — and a `Fault` here would be
            // a line printed the first frame of every fold.
            false => Err(Skip::Transient),
        }
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.target
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    /// **Nothing.** The picture and deck A's cell are textures the panel
    /// samples in the same submission; what reaches a display is the window,
    /// and the window is not a sink here — `karakuri-cli`'s window shows the
    /// canvas, and this one shows the panel.
    fn present(&mut self, _gpu: &Gpu) -> Result<(), String> {
        Ok(())
    }
}

/// **The engine behind the Program bay: a deck of [`SLOTS`] Sets, the present
/// pass, and the two textures it lands in.**
///
/// Scaffolding still in what it is wired to — no audio, no MIDI, no store, no
/// arguments — and a watcher on each slot, which is the one thing here that is
/// not the shortest path to texels and is there because the Staging lane's
/// rows are verdicts on builds ([`watched`]). What `karakuri-cli` does around
/// this is a program; what is here is the shortest path from two `.kir` files
/// to texels — and now back again, which is what a watched slot is.
///
/// **The deck is full, and the slots are channels rather than exhibits.** It
/// has every slot a `Deck` can hold, because a strip is a slot and a mixer is
/// its channels; what is *in* them is this program's one pair at four salts,
/// which is what a slot nobody has loaded anything into holds ([`Engine::new`]).
/// [`ON_AIR`] is Live and is the whole of the picture. Every other slot rests
/// at `Residency::Allocated` — contributing nothing to the mix, and stepped
/// and drawn into its own cell all the same —
/// and [`ASKED_TO_PRIME`] is additionally asked to warm up and parked by the
/// budget in [`Engine::ask_to_prime`], which is what puts a pending request on
/// this panel for the mixer's tally to draw. **The three cost a step and a draw
/// each**, and none of it reaches the governor, which reads a per-Set cost.
/// This sentence has been wrong twice in the same direction — it said the three
/// cost nothing while they were being drawn, and *a draw each and no step*
/// while they were being stepped — so what it is now is the whole of a frame
/// for every slot, which is what
/// [ADR-0269](../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)
/// makes it. The number that goes with it is not one this file can carry: it is
/// `karakuri-engine`'s `tests/deck.rs`, which prints a deck of four against a
/// deck of one on the machine reading it.
///
/// **All four preview cells are on, whatever the decks are doing.** A cell is
/// drawn because there is a slot behind it ([`Engine::aim`]), and this deck is
/// full, so four cells show four slots' own material, all four of them
/// running: deck A stepping on air, deck B warming or parked, C and D warming
/// with nobody having asked. That is [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
/// met on this surface — the operator watches a candidate's cell to decide
/// whether it is worth a fader, and then raises the fader. It used to be gated
/// on `Residency::Live`, which left the three cells worth looking at dark; the
/// gap ADR-0241 named was this line.
pub(crate) struct Engine {
    pub(crate) deck: Deck,
    /// **Elements per geometry, read off the L1's own `capacity` declaration**
    /// rather than named here — see [`Engine::new`]. Kept because the reading
    /// [`Costs::say`] prints names it, and a workload figure that is not the
    /// one the run used is worse than none.
    pub(crate) capacity: u32,
    pub(crate) present: Present,
    /// The Program bay's picture.
    pub(crate) picture: Presented,
    /// **The four deck preview cells**, one per deck slot.
    pub(crate) previews: [Presented; DECKS],
    /// Cached bind groups for each slot view into the tone-mapping pipeline.
    pub(crate) slot_bind_groups: [Option<wgpu::BindGroup>; DECKS],
    /// **The look every sink is drawn under this frame**, and the one piece of
    /// engine state this program *moves*.
    ///
    /// It was [`LOOK`] handed straight to `compose` every frame, with the
    /// reason written at that constant: this file had no session, no `look`
    /// record and no key that changed it. It has a control now — the transport
    /// row's tone map capsule and its exposure track — so a press becomes
    /// `Operation::SetTonemap` or `SetExposure`, which become one
    /// `Record::Look`, which [`apply`] writes here; the next frame hands this
    /// to `compose` and the present pass uploads it. That is P-0090 on this
    /// value exactly: the control ends in the record every other surface's
    /// does, and nothing calls `Present::set_tonemap` behind its back.
    ///
    /// **It lives here rather than beside the panel** because it is what the
    /// *engine* is drawing under: `view::Look` is the console's reading of it,
    /// written per frame from this the way a strip is written from the deck,
    /// and a second copy that the console owned would be the reading and the
    /// state as one thing (ADR-0156).
    ///
    /// `white_point` is carried and never asked for: no surface has a control
    /// for it, so it is read back into every record and written out again
    /// unchanged ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
    pub(crate) look: Look,
    /// **What the master chain is set to**, and [`Engine::look`]'s twin at the
    /// other end of that chain.
    ///
    /// Held here for `look`'s reason exactly: a press becomes
    /// `Operation::SetFeedback`, `SetBloom` or `SetRgbShift`, which become one
    /// `Record::MasterChain`, which [`apply`] writes here; the frame loop puts
    /// it on the `Present` and the slots' uniforms are written from it. Nothing
    /// calls that setter behind the record's back, which is P-0090 on this
    /// value.
    ///
    /// **A list where the out is a bare `f32` on the deck**, and the two are
    /// apart for the reason their records are: the level at the chain's entry
    /// is ridden by a fader and the chain's slots are moved by a press
    /// ([ADR-0317](../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md),
    /// [ADR-0340](../../../docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)).
    ///
    /// **A description and not the built chain**, which is the one thing that
    /// changed when the chain became a list: `karakuri_engine::Present` holds
    /// the compiled slots and is still the only writer of them, and this is
    /// what a record says the chain should be. The frame loop puts one on the
    /// other where the two differ.
    pub(crate) chain: Vec<karakuri_engine::SlotSpec>,
    /// How many registrations have been freed, **over both textures**. The
    /// atlas leak this exists to prevent is invisible from outside: a resize
    /// that registers without freeing leaves a bind group per drag frame and
    /// nothing says so, so the count is kept and `mod gpu` asserts on it. It
    /// is the whole engine's tally rather than either texture's, which is why
    /// it lives here and is handed to [`Presented::fit`].
    pub(crate) freed: usize,
    /// **One [`Aiming`] per slot, in slot order**: how a load or a rewiring
    /// reaches that slot's build worker, and where that watcher is pointed.
    ///
    /// This is the whole of what putting a library Set on a running deck took,
    /// and what it is *not* is the point of it. `Deck::install` is the one
    /// function that puts a built Set in a slot and says of itself that it is
    /// *"deliberately not reachable from a key or a surface: a live run
    /// changes its material by editing a file and letting the worker build it,
    /// which is what the budget watchdog is attached to."* So nothing here
    /// builds a Set: [`loading`] writes the library Set's procedures into the
    /// scratch and sends an aim, and the same worker that watches for a save
    /// picks it up. The swap lands at a frame boundary, is judged there on what
    /// one frame of that Set costs, and rolls back on its own if that is over
    /// the budget — none of which had to be written for the library, because a
    /// load is now literally an edit this program made.
    ///
    /// **In slot order, so the index is the deck letter**: `aimed[0]` is deck
    /// A's, and it is the same index `Deck::events`, the strips and the
    /// preview cells are all in. Kept beside the deck rather than inside it
    /// for the reason the whole of [`Engine`] is on this side: the channel is
    /// `karakuri-environment`'s and the engine takes no environment.
    pub(crate) aimed: Vec<Aiming>,
    /// **The run's wiring** — every edge a `wire_input` has written, for the
    /// whole run and not per slot.
    ///
    /// One list because `--edge` is one list: an edge names the node that
    /// declares the input and what its procedure calls it, and a Set that has
    /// not got that node passes it over where it is built. See [`rewired`].
    ///
    /// **What a rebuild carries and what a save records**, which is why it lives
    /// here rather than inside a watcher: [`Aiming::re_aim`] restates it to the
    /// worker and [`playing_values`] writes it into the file, and those are one
    /// list or they are two answers to what the run is wired with.
    ///
    /// **It lives beside the aims rather than beside the saves**, and that is
    /// what lets a press reach it: a rewiring writes this list and re-aims a
    /// slot, and both halves are here. It was `Keeping`'s until 2026-09-09,
    /// when the Inspector's `uses` line gave the list a second writer that is
    /// not a model's request — see `docs/adr/0329-…`.
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// **Where this deck says which files its slots are running**, for the
    /// readers that are not on this thread — see [`Aiming::pointing`], which is
    /// a clone of this, and [`karakuri_mcp::Slots`].
    ///
    /// **The engine keeps it so that the two readers ask one handle.** The MCP
    /// server was handed the launch working copies and the landing on a row of
    /// the edit history built a second `Slots` of its own out of the aims
    /// (ADR-0308); the first went stale on the first library load and the
    /// second was the workaround for it. There is one now, this is it, and
    /// [`restored`] reads it rather than rebuilding one.
    pub(crate) pointing: karakuri_mcp::Slots,
    /// **Every node the run launched with**, in file order, with the bytes each
    /// one was compiled from — see [`karakuri_environment::compile::Placed`].
    ///
    /// **One list for four slots, because the four files hold the same bytes.**
    /// [`working_copies`] writes the one pair the command line settled into
    /// every slot, so a node's layer, its index, its address and its source are
    /// the same answer four times; the only per-slot difference is the *path*,
    /// which each watcher is given from `slots[slot]` and which no part of a
    /// saved node carries. A second compile per slot would be four answers to
    /// one question with a window between them — see [`Placed::source`], which
    /// is where that hazard is written.
    ///
    /// This is what [`Playing`] is seeded from, and it is the reason a deck can
    /// be saved on the first frame rather than only after something has been
    /// rebuilt.
    pub(crate) placed: Vec<karakuri_environment::compile::Placed>,
}

/// **One slot's watcher, and where it is pointed.**
///
/// `karakuri-cli`'s `Aiming` is the same pair for the same reason, restated
/// here because that program is a binary with no library target and there is
/// nothing to call.
///
/// **The aim is kept and not only the sender**, because a [`watch::Aim`] is
/// every field of the slot's identity and *anything left out comes back as the
/// outgoing slot's* — a fold silently un-selected, a camera back at
/// `Orbit::default()`, salts that repaint every element. A rewiring changes one
/// field of an aim, so all the others have to be restated from somewhere, and
/// this is that somewhere: what the watcher was constructed with until the
/// first aim, and the last aim after that.
///
/// **It is also where the Set a slot is running lives**, which is the roadmap's
/// *per-slot `Option<String>` beside the deck* answered where a per-slot value
/// that must survive a re-aim already lives: [`watch::Aim::set`] is moved by
/// every load and restated by every rewiring, and a second copy on [`Gfx`]
/// would be a second answer to *what is this slot running*
/// (`docs/principles/0087-name-the-property-never-the-shape.md`).
/// [`Gfx::material`] is not that answer and never was — it is the mixer
/// strip's readout, and at launch it is the pair the run was started with.
///
/// **This program had the sender and not the aim**, which was harmless for as
/// long as the only thing that sent one was [`loading`] — a load states every
/// field off the Set file it read. It stops being harmless the moment anything
/// changes *one* field, which is what `wire_input` does: a rewiring that
/// restated the launch pair would have thrown away the Set the operator had
/// just loaded.
pub(crate) struct Aiming {
    /// The other end of [`watch::Watch::aimed_by`]'s channel, for this slot's
    /// watcher and no other. A watcher re-pointed through somebody else's
    /// sender would rebuild a deck nobody named.
    pub(crate) aim: std::sync::mpsc::Sender<watch::Aim>,
    /// Where that watcher is pointed, kept in step with what has been sent.
    pub(crate) at: watch::Aim,
    /// **Where that answer is published for the readers that are not on this
    /// thread**, which today are the MCP server and the landing on a row of
    /// the edit history — see [`karakuri_mcp::Slots`].
    ///
    /// **A publication and not a second answer.** `at` above is the derivation
    /// ([`Aiming`]'s own head); this is a clone of the run's one handle, and
    /// nothing writes it except [`Aiming::publish`], which reads `at`. The
    /// server used to be handed the launch working copies instead, and after a
    /// library load it resolved every address against the files the deck had
    /// stopped running — a `read_procedure` that answered about the wrong
    /// material, a `write_procedure` that wrote where no watcher was looking,
    /// and a node the loaded Set does hold refused for not existing
    /// (`docs/principles/0094-…`, and ADR-0308's *Doubted*, which recorded it
    /// and worked around it for the landing alone).
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Which slot this is, so a publication lands on the row it is about. It is
    /// the index [`Engine::aimed`] is in, which is the deck letter.
    pub(crate) slot: usize,
}

impl Aiming {
    /// **A watcher, and the handle where this slot's files are published.**
    ///
    /// It publishes at construction as well as on every re-point, because a
    /// window remade makes these again and puts every slot back on the pair the
    /// run launched with (ADR-0304): a handle left holding the layout a load
    /// had put there would outlive the deck that was running it.
    pub(crate) fn new(
        aim: std::sync::mpsc::Sender<watch::Aim>,
        at: watch::Aim,
        pointing: karakuri_mcp::Slots,
        slot: usize,
    ) -> Aiming {
        let aiming = Aiming {
            aim,
            at,
            pointing,
            slot,
        };
        aiming.publish();
        aiming
    }

    /// **Say where this watcher is pointed**, from the aim and from nothing
    /// else. One write, after the caller has finished writing files and before
    /// the aim goes out, so a call arriving mid-load sees one layout or the
    /// other and never half of either.
    pub(crate) fn publish(&self) {
        self.pointing.re_point(self.slot, &self.at);
    }

    /// **Point the watcher at what it is already looking at, with one field
    /// changed**, and answer whether it is still there to be pointed.
    ///
    /// # This is the whole of what a control over a slot's shape is
    ///
    /// A [`watch::Aim`] is every field of what a slot *is* — its files, how it
    /// layers its renderers, which one is folded to, its capacity, its seed and
    /// its salts, its camera, its wiring, its grants and the Set it is filed
    /// under. Anything that changes one of them changes what the slot runs, and
    /// there is exactly one way to say so: restate the rest and send the aim.
    /// The worker rebuilds off the render thread, the build lands at a frame
    /// boundary and the watchdog judges it there, on what one frame of that Set
    /// was measured to cost, like every other build (ADR-0228, ADR-0313,
    /// ADR-0314). **Nothing is installed and
    /// nothing on the render thread allocates**, and the values somebody moved
    /// cross the swap (ADR-0282).
    ///
    /// So a *setter on the engine* is not what a control over one of these
    /// fields waits on, and reading `Set::merge`'s or `Set::source_salts`'
    /// absent writer as a blocker is reading the wrong half of the sentence:
    /// the mechanism that exists is the one this instrument already changes
    /// material with
    /// ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
    ///
    /// `change` takes the aim rather than the caller building one, because
    /// [`restated`] is what makes a re-aim safe and it reads `self.at`: a
    /// caller that assembled its own would be the fourteen-field restatement
    /// written a second time, which is exactly the mistake `Watch::repointed`
    /// destructures with no `..` to stop.
    ///
    /// `Err` is a build worker that has ended — the receiver is gone — which is
    /// a run shutting down. It is reported rather than swallowed: the change is
    /// in this program's aim either way, and *nothing will rebuild* is a
    /// different fact from *the slot is recompiling*.
    pub(crate) fn changed(&mut self, change: impl FnOnce(&mut watch::Aim)) -> Result<(), ()> {
        change(&mut self.at);
        // **Said again although a rewiring moves no file**, which is the point
        // of putting it here rather than at the one call that does: this is one
        // of the two places an aim leaves this program, and a publication that
        // covered only the other would be a rule somebody has to remember.
        self.publish();
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }

    /// **The run's wiring, said again**, which is [`changed`](Self::changed)
    /// with the one field a `wire_procedure` moves.
    ///
    /// A method rather than the closure at the call site because `rewired` maps
    /// over slots and a named field is what the reader of that map wants to
    /// see.
    pub(crate) fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.changed(|at| at.edges = edges)
    }

    /// **Point it at something else entirely**, keeping the aim that was sent.
    ///
    /// The one route a load takes, and the reason [`loading`] is handed this
    /// rather than the sender: a load that sent an aim and left `at` behind
    /// would leave the *next* rewiring restating the material the run launched
    /// with, which is the hardest version of this mistake to see.
    pub(crate) fn re_point(&mut self, aim: watch::Aim) -> Result<(), ()> {
        self.at = aim;
        self.publish();
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// One aim, said again — because [`watch::Aim`] is not `Clone` and a re-point
/// restates every field of it.
///
/// **No `..` on either side of this**, which is `Watch::repointed`'s own rule
/// met from the sending end: it destructures with no `..` so that a field added
/// to `Aim` cannot be left behind, and a *sender* that filled the new field
/// with a default would defeat that from here. The compiler names every one of
/// them, so the day another arrives this stops compiling rather than quietly
/// re-aiming a slot at it.
pub(crate) fn restated(aim: &watch::Aim) -> watch::Aim {
    let watch::Aim {
        head,
        rest,
        layering,
        live,
        capacity,
        seed_salt,
        salts,
        camera,
        overrides,
        published,
        bindings,
        edges,
        authorities,
        set,
    } = aim;
    watch::Aim {
        head: head.clone(),
        rest: rest.clone(),
        layering: *layering,
        live: *live,
        capacity: *capacity,
        seed_salt: *seed_salt,
        salts: salts.clone(),
        camera: *camera,
        overrides: overrides.clone(),
        published: published.clone(),
        bindings: bindings.clone(),
        edges: edges.clone(),
        authorities: authorities.clone(),
        // **What this slot is running, carried through a rewiring untouched.**
        // A rewiring changes an edge and not the material, so the versions
        // written after it belong to the same Set as the ones before it —
        // dropped here, a slot would go back to filing under no Set at all on
        // the first `wire_input` after a load.
        set: set.clone(),
    }
}

// ---------------------------------------------------------------------------
// Keeping what a deck is playing
// ---------------------------------------------------------------------------

/// **What each deck is running, as the nodes a Set file names.**
///
/// `karakuri-cli`'s `Running` is the same fact held the same way, and this is
/// the second surface rather than a copy with a different opinion — that
/// program is a binary with no library target, so there is nothing to call.
/// What differs is the shape and only the shape: that one holds addresses and
/// zips the launch bytes back on at save time, and this one holds the
/// [`setfile::SavedNode`] whole, because the panel has one launch list for four
/// slots and nothing to zip it against.
///
/// **It is seeded before the first frame**, from [`Engine::placed`] — so every
/// deck can be written down from the outset rather than only after something
/// has been rebuilt. A slot that is `None` is one whose last build's sources did
/// not reach the store, which the watcher said at the time; it saves nothing
/// rather than guessing.
///
/// **A type of its own rather than a field on [`App`]**, because what a slot is
/// running is one fact with one transition: a build lands and it moves. It held
/// two lists until ADR-0316 — what is playing and what a rollback would bring
/// back — because a rollback was the only thing that could name what it
/// restored. **Nothing restores anything now**, so the second list was a
/// version kept against an event that cannot happen; putting a version back is
/// `Revision::Previous`, which reads the store's history and lands a build like
/// any other, and this then records it like any other.
pub(crate) struct Playing {
    pub(crate) playing: Vec<Option<Vec<setfile::SavedNode>>>,
}

impl Playing {
    /// **Every slot seeded from the material this run compiled**, addressed by
    /// the bytes that compile read.
    ///
    /// **No store, no disk and nothing that can fail.** The bytes ride along in
    /// [`setfile::SavedNode::source`] and reach the store at the moment a file
    /// names them, which is [`setfile::Sources::into_nodes`] — so a run that
    /// never saves writes no artifact.
    pub(crate) fn at_launch(
        placed: &[karakuri_environment::compile::Placed],
        slots: usize,
    ) -> Playing {
        let nodes: Vec<setfile::SavedNode> = placed
            .iter()
            .map(|node| setfile::SavedNode {
                layer: setfile::kind_name(node.layer),
                index: node.index,
                hash: node.hash(),
                name: node.named.name.clone(),
                source: Some(std::sync::Arc::clone(&node.source)),
                meta: Some(std::sync::Arc::clone(&node.meta)),
            })
            .collect();
        Playing {
            playing: (0..slots)
                .map(|_| match nodes.is_empty() {
                    true => None,
                    false => Some(nodes.iter().map(copied).collect()),
                })
                .collect(),
        }
    }

    /// What `slot` is running, or `None` for a slot with no address to name.
    pub(crate) fn at(&self, slot: usize) -> Option<&Vec<setfile::SavedNode>> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// **A build landed.** `nodes` is `None` when that build's sources never
    /// reached the store — the watcher says so at the time, and the addresses it
    /// would have named do not exist.
    ///
    /// **That is still a swap**, and taking it as one is the whole of why this
    /// is a method rather than an assignment at the call site: the slot is on
    /// something new, and it has no address until the next build lands.
    ///
    /// **It is still a swap when the slot is stopped for cost, too.** A version
    /// the watchdog stopped is in the slot and is what a save of that slot must
    /// write down (ADR-0316); what is not true of it is that the slot is
    /// running, which is the lane's to say and not this list's.
    pub(crate) fn landed(&mut self, slot: usize, nodes: Option<Vec<setfile::SavedNode>>) {
        if slot >= self.playing.len() {
            return;
        }
        self.playing[slot] = nodes;
    }
}

/// One saved node, again — because [`setfile::SavedNode`] is not `Clone` and a
/// save consumes the list it is handed while the run goes on holding it.
///
/// The bytes and the card are `Arc`s and are shared rather than duplicated,
/// which is what those two fields are `Arc`s for.
pub(crate) fn copied(node: &setfile::SavedNode) -> setfile::SavedNode {
    setfile::SavedNode {
        layer: node.layer,
        index: node.index,
        hash: node.hash,
        name: node.name.clone(),
        source: node.source.clone(),
        meta: node.meta.clone(),
    }
}

/// **What a build that just landed is running**, from the addresses the watcher
/// reported and the names the slot is spelled with.
///
/// The bytes are `None` for every one of them, and that is
/// [`setfile::SavedNode::source`]'s own rule rather than an omission: the
/// watcher put this build's sources in the store as it built them, so there is
/// nothing left here to carry.
///
/// **The names come off the aim the slot is pointed at**, zipped by position.
/// `watch::Built::nodes` is built from the sort's `Placed`, which keeps file
/// order, and [`Aiming::at`]'s head and rest are that same file list — so entry
/// `n` of one is entry `n` of the other. A name belongs to the *use* rather
/// than to the procedure, so nothing a hash carries could hold it, and the
/// alternative is a Set loaded with named nodes coming back after its first
/// rebuild with the `edge` records pointing at nothing.
pub(crate) fn built_nodes(built: &watch::Built, at: &watch::Aim) -> Vec<setfile::SavedNode> {
    let names: Vec<Option<String>> = std::iter::once(at.head.name.clone())
        .chain(at.rest.iter().map(|node| node.name.clone()))
        .collect();
    built
        .nodes
        .iter()
        .enumerate()
        .map(|(node, (layer, index, hash))| setfile::SavedNode {
            layer,
            index: *index,
            hash: *hash,
            name: names.get(node).cloned().flatten(),
            source: None,
            meta: None,
        })
        .collect()
}

/// **What a Set file says about the Set that is playing**, read off that Set.
///
/// `karakuri-cli`'s `playing_values` is this function and its doc is the
/// argument for every line: **eight of the nine are read from the Set and not
/// from anything this program was told**, because a writer with its own copy of
/// the rule records numbers the run was not using and the file then describes a
/// picture nobody has seen. The capacities are the Set's per geometry, the
/// params are every declaration of every node at the value it is holding, the
/// layering and the fold are the Set's rather than a flag's, and the salts are
/// what it *is* salted with rather than what a position would derive.
///
/// The ninth is `edges`, which is the run's — see [`App::edges`]. It is not the
/// Set's for the reason `mcp::WireRequest` states: the wiring a slot rebuilds
/// with is not on disk anywhere, and the run is the only thing that holds it.
///
/// Restated here rather than called, for [`number_for`]'s reason.
pub(crate) fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        // Filled where a store is open, and nowhere else — see [`Save::run`].
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        // **`Set::orbit` and not the `Set::camera` field**: three of the six
        // are the camera node's parameters, so the field is what was last
        // stated and the map is what a hand, a binding or a carried ride left
        // there (ADR-0318).
        camera: set.orbit(),
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// **Which renderer a Set is folded to**, as a `merge` record spells it:
/// `Some(i)` where exactly one input is live, and `None` where every one of them
/// is.
///
/// **Every-live is checked first, and that decides the one-renderer case.** A
/// composited Set holding a single renderer has one live input, which is both
/// "all of them" and "exactly one" — and it is the first, because such a Set is
/// one nobody has selected in. Writing `live 0` for it would record a choice
/// that was never made.
pub(crate) fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// Whether this deck holds `slot`. The companion of
/// [`karakuri_environment::no_such_slot`], which is the sentence it is refused
/// in.
///
/// **A raw `usize` in and a raw `usize` checked**, on [`held`]'s own terms
/// elsewhere in this file: the callers here (`Keeping::save_set`'s and
/// `Keeping::keep_procedure`'s slot arguments, an MCP request's own number)
/// have no address yet to hand a [`DeckSlot`] — only a number to validate
/// before one can be made. It checks through [`DeckSlot::new`] rather than
/// `slot < slot_count` again, which is the same question [`held`] and
/// `karakuri-cli`'s own `slot_in_range` answer.
pub(crate) fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    u8::try_from(slot)
        .ok()
        .is_some_and(|slot| DeckSlot::new(slot, slot_count).is_some())
}

/// **One live save, from the frame that asked for it to the file on disk.**
pub(crate) struct Save {
    pub(crate) slot: usize,
    /// **Whose act this save is**, which decides the directory it lands in and
    /// is decided at the call site — see [`Keeping::save_set`] and
    /// [`karakuri_environment::Asked`].
    pub(crate) asked: Asked,
    pub(crate) id: String,
    /// The store root, not an open store: opening it creates directories, which
    /// is I/O, which belongs on the thread below rather than on a frame.
    pub(crate) root: std::path::PathBuf,
    pub(crate) sources: setfile::Sources,
    /// **What the file will say, with `nodes` still empty.** The nodes are the
    /// one part of a Set file that needs a store — a hash per source — so they
    /// are filled in where one is opened and never here.
    pub(crate) values: setfile::Owned,
}

impl Save {
    /// Write it. **Everything here is off the render thread**: opening a store
    /// creates directories, and the Set file itself is written and renamed into
    /// place.
    pub(crate) fn run(self) -> Result<(), String> {
        let Save {
            asked,
            id,
            root,
            sources,
            mut values,
            ..
        } = self;
        let store = Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
        // **Before the file that references them**, which is what
        // `setfile::Node` carrying a hash asks of every caller: the writer
        // cannot check that a hash resolves without reading the store back, so
        // putting them is the caller's promise.
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, asked, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
pub(crate) struct Saved {
    pub(crate) slot: usize,
    /// Carried through so the sentence at the end names the right place: the
    /// library's line points at the Library bay, and the sandbox's cannot,
    /// because the bay does not list one.
    pub(crate) asked: Asked,
    pub(crate) id: String,
    /// `Ok` and the file is on disk under `id`. **A failure is printed and
    /// nothing claims otherwise**: a program saying a save happened when the
    /// disk refused is the shape of lie this codebase is arranged against.
    pub(crate) outcome: Result<(), String>,
    /// Where a client that asked for this save is waiting, and `None` when a
    /// hand pressed `k`.
    ///
    /// **It rides the save rather than being looked up when the outcome
    /// lands.** A map from an id to whoever asked would be a second place that
    /// knows which save is which, and the outcome already carries everything
    /// needed to find its way home.
    pub(crate) reply: Option<mcp::Reply>,
}

/// **One node's procedure kept**, from the frame that asked for it to the file
/// on disk and back.
///
/// [`Save`] and [`Saved`] folded into one type, and that is the difference in
/// the act rather than a shortcut: a Set save gathers a whole slot's worth of
/// readings off the live deck, so what is *asked for* and what *came back* are
/// two shapes; a keep is one node's bytes and a name, so the request and the
/// outcome carry the same three fields and the outcome is what is added.
///
/// **Nothing here is a reading of the engine.** The bytes are the run's own —
/// what [`Playing`] holds for that slot — and where they go is decided by
/// [`Asked`], which is the call site's. So the whole of this crosses onto the
/// write thread with no deck behind it.
pub(crate) struct Kept {
    /// **Whose act this keep is**, which decides the directory it lands in —
    /// [`Save::asked`]'s field and its argument: an operator's own act writes
    /// `<store>/procedures/` and a model's writes `<store>/sandbox/`
    /// (P-0096, ADR-0261).
    pub(crate) asked: Asked,
    /// **What it is filed as** — the name typed into this pane's head where
    /// one was, and a stamp where the capsule typed nothing (ADR-0128,
    /// ADR-0287, ADR-0292).
    pub(crate) name: String,
    /// The store root, not an open store — [`Save::root`]'s reason: opening it
    /// is I/O and belongs on the thread below.
    pub(crate) root: std::path::PathBuf,
    /// **The bytes, where this node is still on the version the run launched
    /// with**, and `None` where a build put it in the store instead — which is
    /// [`setfile::SavedNode::source`]'s own rule. Either way the address below
    /// names it, so the write thread has one place to go for what it has not
    /// got.
    pub(crate) source: Option<std::sync::Arc<str>>,
    /// **The address of those bytes**, which is what a rebuilt node carries
    /// instead of them: the watcher put its source in the store as it built
    /// it, so `<hash>.kir` at the store root is where a keep reads it back
    /// from.
    pub(crate) hash: karakuri_store::hash::Hash,
    /// **What the pane calls this node** — `L2:0` — carried for the sentence
    /// and nothing else. An address is what an operator is looking at when
    /// they press, and an outcome naming a file with no node beside it is an
    /// answer to a question nobody asked.
    pub(crate) addr: String,
    /// Where the file went, once it has gone there — `Ok` and it is on disk,
    /// `Err` and nothing claims otherwise, which is [`Saved::outcome`]'s rule.
    pub(crate) outcome: Result<std::path::PathBuf, String>,
    /// Where a client that asked for this keep is waiting, and `None` when a
    /// hand pressed the capsule — [`Saved::reply`]'s field and its reason.
    pub(crate) reply: Option<mcp::Reply>,
}

impl Kept {
    /// **Write it.** Everything here is off the render thread: opening a store
    /// creates directories, an artifact may have to be read back, and the file
    /// itself is written and renamed into place.
    ///
    /// **The bytes are found in one of two places and never a third.** A node
    /// still on its launch version carries them; a node a build landed has
    /// them in the store under the address it carries instead. **Neither is a
    /// re-read of the `.kir` on disk**, which is
    /// [`setfile::SavedNode::source`]'s own sentence: a file rewritten since
    /// the compile is a version nobody has seen, and a keep of it would put a
    /// picture nobody watched into a library.
    pub(crate) fn run(self) -> Kept {
        let Kept {
            asked,
            name,
            root,
            source,
            hash,
            addr,
            reply,
            ..
        } = self;
        let outcome = (|| {
            let store =
                Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
            let bytes = match &source {
                Some(source) => source.as_bytes().to_vec(),
                None => store
                    .get_artifact(&hash)
                    .map_err(|e| format!("the source of `{addr}`: {e}"))?,
            };
            match asked {
                Asked::Operator => store.write_procedure(&name, &bytes),
                Asked::Model => store.write_sandbox_procedure(&name, &bytes),
            }
            .map_err(|e| e.to_string())
        })();
        Kept {
            asked,
            name,
            root,
            source,
            hash,
            addr,
            outcome,
            reply,
        }
    }

    /// **The one sentence this outcome is said in**, formed here so that the
    /// words a test reads, the words an operator reads and the words a model
    /// is handed are the same run of text — [`Sent::said`]'s rule and
    /// [`Keeping::took_save`]'s.
    pub(crate) fn said(&self) -> Result<String, String> {
        match &self.outcome {
            Ok(path) => Ok(match self.asked {
                Asked::Operator => format!(
                    "  keep: {} kept as procedure `{}` — `{}`. The Library bay lists it, and a \
                     press on that row writes it over that layer of what a deck is playing",
                    self.addr,
                    self.name,
                    path.display()
                ),
                // **The sandbox's sentence says where it is and what does not
                // read it**, which is a Set save's own division one file kind
                // along: a model told only that a procedure was kept would go
                // looking for a library row that is not there (P-0083,
                // ADR-0261).
                Asked::Model => format!(
                    "  keep: {} kept as `{}` in the sandbox — `{}`. A keep asked for over MCP is \
                     written there rather than in the operator's library, so the Library bay does \
                     not list it and no load off a row reaches it",
                    self.addr,
                    self.name,
                    path.display()
                ),
            }),
            Err(e) => Err(format!(
                "  keep: {}: procedure `{}` was not kept: {e}",
                self.addr, self.name
            )),
        }
    }
}

/// **What a send came back with**, at the frame it arrives.
///
/// [`Saved`]'s shape one act along, and the fields differ where the two acts
/// do: a send files under no id in this store, so there is no `Asked` to carry
/// — the answer to *whose library is this* is *nobody's*, which is the whole
/// of what sending is — and there is no `mcp::Reply`, because no tool asks for
/// one.
pub(crate) struct Sent {
    /// **The Set that was packaged**, which is the row the menu was opened on.
    pub(crate) id: String,
    /// **Where the operator sent it**, or `None` where they dismissed the
    /// dialog without naming anywhere.
    ///
    /// **`None` is an outcome and not a failure**, which is why it is here
    /// rather than an `Err` in [`outcome`](Self::outcome): nothing went wrong,
    /// nothing was written, and the sentence a reader needs is the third one
    /// rather than a refusal (P-0083 is about what a *rejection* carries, and
    /// this is not one).
    pub(crate) to: Option<std::path::PathBuf>,
    /// `Ok` and the bundle is on the disk at [`to`](Self::to). **A failure is
    /// printed and nothing claims otherwise**, which is [`Saved::outcome`]'s
    /// own rule.
    pub(crate) outcome: Result<(), String>,
}

impl Sent {
    /// **The one sentence this outcome is said in**, formed here so that the
    /// words a test reads and the words an operator reads are the same run of
    /// text — [`Keeping::took_save`]'s *one sentence for both audiences*, with
    /// one audience.
    ///
    /// **Three outcomes and three sentences.** Written; refused, naming what
    /// the disk or the store said; and *no file was named*, which is not a
    /// refusal and does not read like one — nothing went wrong, and rule 04 of
    /// the manual is that a press that did nothing says so rather than going
    /// quiet.
    pub(crate) fn said(&self) -> String {
        let Sent { id, to, outcome } = self;
        match (to, outcome) {
            (None, _) => format!(
                "  send: `{id}` was not written — the save dialog was dismissed, and nothing was \
                 asked of the disk"
            ),
            (Some(to), Ok(())) => format!("  send: `{id}` written to `{}`", to.display()),
            (Some(to), Err(e)) => {
                format!("  send: `{id}` was not written to `{}`: {e}", to.display())
            }
        }
    }
}

/// A save that will not happen, to the terminal and to whoever asked if that was
/// not a hand.
///
/// **One sentence and one home.** Every refusal here reaches two audiences, and
/// the way that goes wrong is a copy of the words for the second one — free to
/// be right on the day it is written and wrong at the next correction.
pub(crate) fn refused(reply: Option<mcp::Reply>, said: String) {
    println!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}

impl Engine {
    /// **Sized from the arrangement rather than from the window**, by the same
    /// two calls the frame aims with — see [`aims`]. The window this opens at
    /// gives the picture and deck A's cell their first rectangles, so no frame
    /// has to correct a guess and there is no second derivation here to drift
    /// from the one in [`Engine::aim`].
    // Eight, for [`watched`]'s reason: the last three are the run-wide handles
    // this constructor hands every watcher it makes, and each has a different
    // owner in [`main`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        slots: &[Sources],
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **Where every watcher puts what it builds, and where it reports it.**
        // `None` is a harness with no store to write into, which is every test
        // under `mod gpu` below: nothing there saves, and a run that created a
        // store to draw four cells would be the side effect
        // `karakuri_environment::scratch` refuses for a `--render`.
        stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
        // **The run's one edit history**, made and seeded in [`main`] and
        // handed to every watcher this makes — see [`watched`]. `None` is a
        // harness with no store, which is every test under `mod gpu` below, on
        // `stored`'s terms exactly.
        //
        // **One `Shared` for the run and not one per window.** A window remade
        // makes these watchers again, and a second `Snapshots` would have an
        // empty dedup memory: the first rebuild after a remake would file every
        // untouched procedure as a new version.
        snapshots: Option<history::Shared>,
        // **Where this deck says which files each of its slots is running**,
        // handed in rather than made here: [`main`] gives the same handle to
        // [`karakuri_mcp::serve`] and to this, so a model's
        // address and the file a watcher is polling are one answer — which is
        // exactly the arrangement the opening already has. It is not
        // `Option` and does not depend on `--mcp`, because the landing on a row
        // of the edit history reads it too ([`restored`]).
        pointing: karakuri_mcp::Slots,
    ) -> Engine {
        assert!(
            slots.len() == SLOTS,
            "a deck of {SLOTS} slots was handed {} pairs to run from",
            slots.len()
        );
        // **Parsed once and built [`SLOTS`] times, and that is a fact rather
        // than an assumption now.** Every entry in `slots` is a copy of the
        // one pair the command line settled ([`working_copies`]), so the four
        // files hold the same bytes at startup and one `Checked` is the same
        // answer four times. What is *not* the same is the path each slot's
        // watcher polls, which is the whole of what per-slot copies buy and is
        // read off `slots[slot]` in the loop below.
        // **Compiled through the sort every other surface compiles through**,
        // which is what this used to do by hand and is the whole of what a save
        // needed: `checked` gave back a `Checked` and dropped the bytes it read,
        // and a node's address is a function of exactly those bytes
        // ([`karakuri_environment::compile::Placed::source`]). Re-reading the
        // path later to hash it is the defect that function's own doc is
        // written against — between here and the first frame sit a device, four
        // `Set::build`s and, now, an MCP server.
        //
        // **Once, for slot 0, and used by all four.** See [`Engine::placed`].
        let (material, placed) = karakuri_environment::compile::sort_slot(
            ON_AIR,
            &karakuri_environment::compile::Named::bare(&slots[ON_AIR].l1),
            &[karakuri_environment::compile::Named::bare(
                &slots[ON_AIR].l4,
            )],
        );
        let l1 = material
            .l1s
            .first()
            .expect("the launch pair declares a geometry")
            .clone();
        let l4 = material
            .l4s
            .first()
            .expect("the launch pair declares a renderer")
            .clone();
        let capacity = capacity_of(&l1);
        // **The same material in every slot, at its own salt and in its own
        // file.** A slot cannot hold *nothing*: `HotSwap::new` takes a live
        // `Set` and `Deck::new` takes one `HotSwap` per slot, so an empty slot
        // is not a state this engine has and the nearest thing to it is a slot
        // holding material nobody has asked for. What this program has to give
        // them is one pair — [`Sources`] is the whole command line — so each
        // gets it at its own salt ([`slot_salt`]): four slots of one procedure
        // at four seeds are four simulations, and four slots at one seed would
        // be one picture drawn four times, which is not a mixer either.
        // Building three of them from other `.kir` files would be this program
        // choosing material for the operator, which is the library's job and
        // not a constructor's
        // ([ADR-0228](../../../docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)).
        //
        // **The salt is no longer the only difference, and that is the fix.**
        // Each slot runs from its own copy of that pair ([`working_copies`]),
        // so the four are the same *material* and four different *files* — an
        // edit reaches the deck whose file it is.
        let built = |salt| {
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, salt)
                .expect("the pair builds a Set")
        };
        // **A watcher per slot, over that slot's own two files**, which is
        // `karakuri-cli`'s wiring and not a second one — one `HotSwap::new`
        // over a `watch::Watch`, at the engine's own default budget.
        //
        // **`slots[slot]` and not one pair repeated**, which is the sentence
        // this comment used to be the other way round. It said every slot was
        // spelled with the same pair and quoted `watch`'s *"Two slots given
        // the same files both rebuild, which is right: the same edit reached
        // both of them"* — true of that module, and the wrong thing for this
        // program to be doing, because it made one save rebuild four slots and
        // fill the Staging lane with three rows that can never leave (a parked
        // slot's trial is frozen, so it reaches no verdict). Each watcher now
        // polls the copy made for its own slot, and no watcher can see
        // another's file at all, which is what `watch`'s *"a slot is the unit
        // that gets replaced"* asks for.
        //
        // **This is the Staging lane's producer**, and it is the whole of what
        // it took. `HotSwap::fixed` keeps a `Receiver` whose `Sender` was
        // dropped at construction, so nothing is ever installed and no
        // `swap::Event` of any variant is emitted — which is why the lane drew
        // its empty state and could reach no other. Nothing about the frame
        // path changed: `install_if_ready` polls the same channel with
        // `try_recv` either way, and everything a rebuild costs — the file
        // read, the four validation stages, the compile and `Set::build` — is
        // on the worker thread this spawns (P-0091).
        //
        // **In slot order, and the loop is the whole of what four slots
        // took**: a `Vec` of `HotSwap` is what `Deck::new` has always taken,
        // and `Engine::aimed` is documented as being in the same order the
        // strips and the preview cells are.
        let mut swaps = Vec::with_capacity(SLOTS);
        let mut aimed = Vec::with_capacity(SLOTS);
        for (slot, running) in slots.iter().enumerate().take(SLOTS) {
            let salt = slot_salt(slot);
            let (swap, aim) = watched(
                gpu,
                running,
                built(salt),
                slot,
                salt,
                stored
                    .as_ref()
                    .map(|(store, tx)| (std::sync::Arc::clone(store), tx.clone())),
                pointing.clone(),
                snapshots.clone(),
            );
            swaps.push(swap);
            aimed.push(aim);
        }
        let mut deck = Deck::new(&gpu.device, swaps, CANVAS.0, CANVAS.1);
        // **Every slot but deck A rests at `Allocated`, which is what a
        // channel nobody has asked anything of is.**
        //
        // `Deck::new` brings every slot up Live, and that is right for a deck
        // built to *play* what is in it — a deck of one is then a bare Set,
        // bit for bit (ADR-0038). This deck is built **full** rather than
        // built to play four, so leaving them Live would put three
        // simulations nobody asked for on the render thread and three layers
        // nobody asked for into the fold: the reading [`Costs::say`] prints
        // would stop being one Set a frame, and every slot comes up under
        // `Blend::Add` at unity, so what the Program bay drew would be four
        // simulations summed — the same material at four times its exposure,
        // which is a mixer set wrong rather than a mixer.
        //
        // `Residency::Allocated` is the state the engine already has for this,
        // rather than one invented here — *off air, asked of nothing* — and
        // `deck::Frame::render` reads the effective residency into the
        // composite's `live` flag, so an allocated slot contributes nothing to
        // the mix. It is drawn (ADR-0258) and it steps (ADR-0269), so what it
        // costs is its buffers, its L1 and its L4, and what it does not cost is
        // a term in the fold. The strip reads ALLOC and the cell reads material
        // running. That is
        // [P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md):
        // *a slot with nothing in it* is a residency this deck already has a
        // word for.
        //
        // **It is the request that is written**, which is the operator's half
        // and the same half [`Engine::ask_to_prime`] writes — see
        // `Deck::set_residency`. So an operator brings a channel up by cycling
        // its tally chip or by loading material into it, and nothing has to
        // undo a decision this constructor made. The governor is told nothing
        // by it either: a slot whose request is Allocated is `Reason::OffAir`,
        // which is *the governor was not asked about this slot*.
        for slot in 0..SLOTS {
            if slot != ON_AIR {
                deck.set_residency(EngineSlot(slot as u8), Residency::Allocated);
            }
        }
        // **The meters are on, and that is a decision rather than a default.**
        // Five of the six things a mixer strip shows are settings the deck was
        // told; the meter is the only one that is a *measurement*, so with it
        // off this bay would draw five readouts that never move beside a well
        // that is always empty — which is the scaffolding-that-looks-finished
        // this panel refuses, read from the other side. It costs a pipeline,
        // two buffers and a ring of staging buffers per slot, allocated here
        // and never on the render thread, which is the same terms `Deck::new`
        // above is on; there are [`SLOTS`] slots, so it is four of each. The
        // three that are not Live report no level — `Deck::level` is `None`
        // for a slot that is not being drawn — which is the meter saying what
        // it measured rather than a strip with a gap in it.
        deck.enable_meters(&gpu.device);
        let present = Present::new(&gpu.device, PICTURE_FORMAT, CANVAS.0, CANVAS.1);
        let (picture_at, preview_ats) = aims(layout, present.size());
        let picture = Presented::new(gpu, renderer, "program view", picture_at, scale);
        let previews = [
            Presented::new(
                gpu,
                renderer,
                "deck A preview",
                preview_ats.and_then(|c| c.first().copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck B preview",
                preview_ats.and_then(|c| c.get(1).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck C preview",
                preview_ats.and_then(|c| c.get(2).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck D preview",
                preview_ats.and_then(|c| c.get(3).copied()),
                scale,
            ),
        ];
        let slot_bind_groups = std::array::from_fn(|slot| {
            deck.slot_view(EngineSlot(slot as u8))
                .map(|view| present.create_bind_group_for(&gpu.device, view))
        });
        Engine {
            deck,
            // **Nothing wired by hand yet**, which is the state a run begins
            // in: the launch aims carry whatever `--edge` said, and this is the
            // list a rewiring writes and every later re-aim restates.
            edges: Vec::new(),
            capacity,
            present,
            picture,
            previews,
            slot_bind_groups,
            look: LOOK,
            // **Nothing in it at all**, which is the default chain: with no
            // slot the mix writes straight into the target the present pass
            // reads, so the frame this program opens on is the frame it drew
            // before the chain existed — bit for bit and for free.
            chain: Vec::new(),
            freed: 0,
            aimed,
            pointing,
            placed,
        }
    }

    /// **Ask deck B to warm up, and let the budget answer.** The one governor
    /// pass this program makes, taken at startup where the stall it costs is
    /// free, and the whole of why a strip on this panel can read one residency
    /// and have been asked for another.
    ///
    /// **The other two slots are not in this, and that is the change.** Deck B
    /// used to be the only other slot there was, so *the deck has a second
    /// slot* and *the panel can show a park* were one sentence; they are two
    /// now. C and D rest at `Residency::Allocated` — [`Engine::new`] says why
    /// — were asked for nothing, and come back from the pass as
    /// `Reason::OffAir`, which is the governor reporting that it was not asked
    /// about them. They cost the arithmetic below nothing: `committed_ms` is
    /// the sum over **Live** slots and deck A is the only one, so this sets
    /// the same budget it set with two slots, off the same measurement, for
    /// the same reason.
    ///
    /// **It is `karakuri-cli`'s order rather than a second one**: measure every
    /// slot before anything is decided about any of them, ask through
    /// [`Deck::set_residency`], and call [`Deck::govern`], which is the only
    /// thing in the engine that writes an *effective* residency. `set_residency`
    /// writes the request **and grants it**, so a harness that never governs
    /// has a deck whose two residencies agree on every slot and every frame —
    /// which is what this file was, and is why the roll ADR-0190 drew was
    /// tested and unreachable. The report comes back whole for the same reason
    /// `karakuri-cli`'s `report_governing` prints one: nothing is printed in
    /// the engine, so what an operator reads and what a test asserts are the
    /// same values.
    ///
    /// # The budget is what moves, and it is moved from what was measured
    ///
    /// A deck starts on `governor::DEFAULT_COMPUTE_BUDGET_MS` — one 60 Hz
    /// frame of measured per-Set cost — and what one of these Sets measures at
    /// is this machine's business rather than anything this file can know. So
    /// the budget is set **from the measurement that was just taken**: what
    /// deck A is already committed to, plus **half** of what warming deck B was
    /// measured at. Half of a cost is not that cost, so the request cannot fit
    /// — the refusal is arithmetic on every machine rather than on the ones
    /// where the numbers happen to come out.
    ///
    /// **It used to be an eighth of it, and the change is ADR-0269's.** The
    /// governor could once admit a slot at one step in `SLOWEST_PRIME_ONE_IN`
    /// frames and charge `cost / n` for it, so parking a request meant leaving
    /// headroom under `cost / 8`. There is no rate left to undercut: a drawn
    /// slot steps every frame, every slot is drawn, and a request either fits
    /// at its whole measured cost or is parked.
    ///
    /// **A number computed from the measurement rather than a constant**,
    /// because a constant is the fixture the product cannot produce
    /// (`docs/contributing.md` §3, *a check you have not watched fail is
    /// guessing*, read from the other side): a budget typed in here parks the request on
    /// this machine and admits it on a faster one, and a *measurement* typed in
    /// — `HotSwap::set_measured_cost` is public and would take one — is this
    /// file writing down the number the probe exists to take.
    ///
    /// **Nothing here writes a residency, a strip or a park.** The deck is
    /// asked and the governor answers; [`mixer`] reads both residencies back
    /// off the deck the way it reads the gain, and `view::Strip::pending`
    /// derives the disagreement. `Deck::is_parked` is not called in this file
    /// at all outside `mod gpu`.
    ///
    /// Where a measurement is missing the budget is left where it was, and the
    /// governor parks the request anyway for a different and more serious
    /// reason — an unmeasured Live slot means the committed cost is unknown,
    /// which suspends priming wholesale. The caller prints the reason it got
    /// rather than the one this comment expects.
    pub(crate) fn ask_to_prime(&mut self, gpu: &Gpu) -> Report {
        // **Before the first frame, and this is the only place it can be.**
        // Measuring means stepping and ends in a rewind, so `measure_slots`
        // skips any slot whose Set has already run — a deck measured late
        // stays unbudgetable rather than losing what it has simulated.
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        // **And the estimate beside the measurement**, on the same terms: two
        // draws per cold slot, before the first frame and never on the render
        // thread. The governor budgets on this where it answers and on the
        // measurement where it does not
        // ([ADR-0296](../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)),
        // and it is taken at the deck's own size, so a deck on a small output
        // stops being judged against a frame nobody is drawing.
        self.deck.estimate_slots(&gpu.device, &gpu.queue);
        self.deck
            .set_residency(EngineSlot(ASKED_TO_PRIME as u8), Residency::Priming);
        // **The number the governor will spend for this slot**, which since
        // ADR-0303 is not the same quantity as its measurement.
        //
        // This read the measurement alone and could not any more: the
        // measurement is taken at the size this file names — the preview cell,
        // which is what a slot is auditioned in — and the governor spends the
        // *estimate* at the output's size wherever one answers (ADR-0296). A
        // budget stated in one of those and spent in the other is not a
        // comparison, and on this machine it is not a small error either: the
        // reference workload estimates 45 ms at 1280x720 and measures 12 at a
        // 252x142 cell, so a budget off the measurement puts a single live
        // slot permanently over it.
        //
        // So the budget is taken on the same precedence the governor decides
        // on — estimate where it answers, measurement where it refuses. That
        // is not a new policy; it is ADR-0296's, applied to the budget's own
        // currency so that both sides of the comparison are about one frame.
        // **Whether that is the right repair is the maintainer's**, and the
        // two alternatives are in ADR-0303: extrapolating the measurement, or
        // stating the budget per size.
        let budgeted = |slot: &HotSwap| {
            slot.estimated_cost()
                .and_then(Estimate::ms)
                .or_else(|| slot.measured_cost().map(|cost| cost.ms))
        };
        if let (Some(committed), Some(warming)) = (
            budgeted(self.deck.slot(EngineSlot(ON_AIR as u8))),
            budgeted(self.deck.slot(EngineSlot(ASKED_TO_PRIME as u8))),
        ) {
            self.deck.set_compute_budget_ms(committed + warming / 2.0);
        }
        self.deck.govern()
    }

    /// **Aim every sink at its own rectangle, and hand back what the console
    /// should draw in each** — the picture, and one entry per preview cell.
    ///
    /// # A cell is aimed because there is a slot behind it, and residency has
    /// nothing to do with it
    ///
    /// This read `Deck::preview` once and aimed the one preview sink at the
    /// cell of the deck the output was auditioning: the sinks all took the same
    /// composited frame, so a sink left in deck A's cell would have drawn deck
    /// C's material under the letter `A` the moment somebody auditioned C.
    /// ADR-0240 retired the audition — the picture is the master mix and every
    /// cell is its own deck's monitor — and the sinks stopped taking the
    /// composited frame: each cell is drawn from `Deck::slot_view` for the slot
    /// it is lettered for, which is a texture that cannot be of the wrong deck.
    ///
    /// **Then it gated the aim on `Residency::Live`, and that was the defect
    /// this pass removes.** The reason given was that an off-air slot "is not
    /// stepping and has nothing new in its view", which was true only because
    /// the engine refused to draw one. It is the exact case
    /// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
    /// exists for: an operator decides whether to put a candidate on air by
    /// watching its cell, and a cell that is dark until the candidate is
    /// already on air answers the question after it stops being asked. The deck
    /// draws every slot into its own target on every frame now, so there is
    /// something new in every view, every frame.
    ///
    /// **What is left to decide is whether there is a slot at all**, and that
    /// is `slot_bind_groups[slot]`: `Deck::slot_view` is `None` past
    /// `slot_count`, so a deck of fewer than [`DECKS`] slots leaves the surplus
    /// cells with nothing to sample. **The aim asks the same question the draw
    /// asks**, so the two cannot disagree — a cell aimed but not drawn would be
    /// a texture from an earlier frame held under a letter, and a cell drawn
    /// but not aimed is a pass into nothing.
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **The projector window's size, or `None` while it is closed.** The
        // second output, and the second half of what [`render_size`] is a
        // maximum over — handed in because the window is `Gfx`'s and this is
        // the engine.
        projector: Option<(u32, u32)>,
    ) -> (Option<Picture>, [Option<Picture>; DECKS]) {
        let (picture_at, preview_ats) = aims(layout, CANVAS);
        let picture = self
            .picture
            .aim(gpu, renderer, picture_at, scale, &mut self.freed);
        // Which cells have a slot behind them, read once for the frame: the
        // loop below both aims and reports off the same answer, and the draw in
        // `compose` reads the same `slot_bind_groups`.
        let behind: [bool; DECKS] =
            std::array::from_fn(|slot| self.slot_bind_groups[slot].is_some());
        let mut previews = [None; DECKS];
        let freed = &mut self.freed;
        for (slot, (sink, out)) in self
            .previews
            .iter_mut()
            .zip(previews.iter_mut())
            .enumerate()
        {
            let at = match behind[slot] {
                true => preview_ats.and_then(|cells| cells.get(slot).copied()),
                false => None,
            };
            let pic = sink.aim(gpu, renderer, at, scale, freed);
            if behind[slot] {
                *out = pic;
            }
        }
        // **What a slot's measurement is a measurement of**, told to the deck
        // here because this is the statement that knows it.
        //
        // This application has two resolutions and no third one: the output
        // size, which the mix is composited once at and which the picture and
        // every other sink is a resize of (ADR-0247), and the size of a deck
        // cell, which is what a slot is auditioned in. A per-slot measurement
        // used to be taken at a constant 1280x720 — a size nothing renders at
        // — and ADR-0303 removed it, so the size is named by whoever knows
        // the layout, which is this file and not the engine.
        //
        // **The first aimed cell, and every slot is told the same one.** The
        // four cells are one row of equal boxes and a probe measures a deck
        // with one target; a per-slot size would make four slots' numbers
        // incomparable, which is precisely what a governor summing them must
        // not have. With no cell aimed at all — the Program bay folded away —
        // nothing is said and the last size stands, because a bay that is not
        // laid out is not a statement that a measurement is about nothing.
        //
        // **Once a frame, and it is a store rather than a measurement.** The
        // cell moves when a divider moves or a bay folds, and the build worker
        // reads this per build, so a size taken once at startup would measure
        // every candidate of a session against whatever the window opened at.
        let cell = self.previews.iter().find(|p| p.aimed).map(|p| p.size);
        if let Some(cell) = cell {
            self.deck.set_measure_size(cell);
        }

        // **The frame's own size, derived from the outputs that are on** —
        // ADR-0247, and [`render_size`] is where the rule is. It is taken here
        // rather than anywhere else because this is the statement that has
        // just decided the picture's rectangle, and the picture's rectangle
        // *is* its size as an output.
        //
        // **A reallocation, so it is the frame's first act and not something
        // done mid-pass**, which is [`Presented::fit`]'s sentence one level
        // out: `aim` runs before `compose`, and P-0091 is about the render
        // thread. What it costs was measured on 2026-09-09 rather than
        // argued — **0.145 ms** for `Deck::resize` and `Present::resize`
        // together on a four-slot deck, host clock, biased high, and within a
        // few percent of the same at 466x262 and at 1920x1080 because what it
        // pays for is nine objects rather than their texels. It is paid on the
        // frames a size changed and on no others: every frame of a divider
        // drag on the Program bay's height is one of them, and it is the same
        // frame `Presented::fit` was already remaking the picture's texture
        // on.
        //
        // **Both, always, and in one statement.** `Frame::render` checks its
        // sizes and panics at the call site, so a deck resized without the
        // present pass is a loud failure a frame later — which is the right
        // failure and the wrong place to find out.
        let outputs = [self.picture.aimed.then_some(self.picture.size), projector];
        if let Some(at) = render_size(&outputs) {
            if at != self.present.size() {
                // **Nothing is printed here**, and that is the frame path
                // rather than reticence: every frame of a divider drag on the
                // Program bay's height changes this size, so a line would be
                // sixty a second and sixty formats a second with it. What the
                // frame is composited at is a *readout* — `Costs::say` prints
                // it beside the numbers it is about, which is what P-0095 asks
                // of a measurement, and the console page's own size pill is
                // where an operator reads it.
                self.present.resize(&gpu.device, &gpu.queue, at.0, at.1);
                self.deck.resize(&gpu.device, at.0, at.1);
                // **And every cell's bind group, because `Deck::resize`
                // replaced the views they were made from.**
                //
                // A bind group holds its view alive, so a stale one samples a
                // texture nothing draws into any more: the cells would go on
                // showing the last frame at the old size, under letters
                // saying their decks are running. That is the silent wrong
                // picture P-0094 refuses, and it is exactly the symptom
                // `Deck::resize`'s own comment names for the meters one line
                // along. It was found by
                // `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`
                // the first time the deck was resized from here.
                self.slot_bind_groups = std::array::from_fn(|slot| {
                    self.deck
                        .slot_view(EngineSlot(slot as u8))
                        .map(|view| self.present.create_bind_group_for(&gpu.device, view))
                });
            }
        }
        (picture, previews)
    }
}

/// Which rectangle each of the engine's sinks is sized from and drawn into.
pub(crate) fn aims(
    layout: &karakuri_layout::Layout,
    canvas: (u32, u32),
) -> (Option<egui::Rect>, Option<[egui::Rect; DECKS]>) {
    (picture_rect(layout, canvas), preview_rects(layout, canvas))
}

/// **The one size the frame is composited at: the largest enabled output's.**
///
/// Each entry is one output — `Some(size)` for one that is on, `None` for one
/// that is off — in the order the Outputs row draws them, which is the program
/// view first. `None` back means **no output is enabled at all**, and the
/// caller leaves the size where it is rather than picking one.
///
/// # Largest by area, and the answer is always some output's own size
///
/// [ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)
/// says *the largest enabled output's size*, so that every output is a
/// downscale and none is ever upscaled. With two outputs of different shapes
/// that needs a rule, and there are two candidates. **A componentwise maximum
/// is refused**: 1920x1080 beside 1024x1280 would give 1920x1280, which is a
/// size no output is and nobody asked for, and it upscales both of them in one
/// axis while claiming to prevent upscaling. **Largest by pixel count wins**,
/// and it hands back one output's own size — so the frame is always exactly
/// right for at least one destination and a downscale for the rest. The tie
/// goes to the earlier entry, which is the program view, because the picture
/// is the output an operator is looking at while they decide.
///
/// # Every output off leaves the size where it was
///
/// **An output going off is not a statement about what the frame should be
/// rendered at.** Turning the last one off stops the publishing and not the
/// instrument
/// ([ADR-0171](../../../docs/adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)),
/// and re-deriving a size there would reallocate every slot target, the mix
/// and the master chain in order to change a picture nobody is looking at —
/// and reallocate them again on the way back. So the fallback to
/// [`CANVAS`] is the run's **starting** value rather than a re-derivation, and
/// this function says *nothing to say* instead of guessing.
///
/// **The rejected reading is ADR-0247's own sentence taken literally** —
/// *"the session's `canvas` record is what it falls back to when no output
/// says anything"* — which is right about a run that has never had an output
/// and wrong about one whose outputs are all momentarily off, and cannot tell
/// the two apart from inside this function. The starting value is where that
/// sentence is honoured; see [`Engine::aim`].
pub(crate) fn render_size(outputs: &[Option<(u32, u32)>]) -> Option<(u32, u32)> {
    // **A fold rather than `max_by_key`**, which returns the *last* of several
    // equal maxima and would give the tie to whichever output happened to be
    // drawn furthest right. `>` and not `>=` is the tie rule said once.
    outputs.iter().flatten().copied().fold(None, |best, at| {
        let area = |(w, h): (u32, u32)| (w as u64) * (h as u64);
        match best {
            Some(b) if area(b) >= area(at) => Some(b),
            _ => Some(at),
        }
    })
}

/// **One tone-mapping pass per deck preview cell, off that slot's own target,
/// whatever the slot's residency.**
///
/// The engine drew every slot into its own target a moment before this — Live
/// ones stepped and drawn, off-air ones drawn and not stepped — so this reads
/// [`DECKS`] fresh images and never the composite. Through the same [`Present`]
/// the picture goes through, so a cell is the material under the transfer curve
/// the room gets, with no fader on it, because a fader is an edge property
/// applied in the mix and a slot's target is upstream of the mix. That is
/// what [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
/// asks of a monitor, in one loop.
///
/// **Gated on `aimed` and on there being a slot to sample, and on nothing
/// else.** Residency was the third gate and is not: a cell that goes dark when
/// its deck goes off air is dark at exactly the moment an operator is deciding
/// whether to bring it back. [`Engine::aim`] asks the same two questions, so a
/// cell cannot be aimed and not drawn.
///
/// A function rather than the body of the closure it is called from, for
/// [`live`]'s reason: `window_event` cannot be called from a test, so the part
/// worth asserting is lifted to where one can reach it — see
/// `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`.
pub(crate) fn monitor(
    present: &Present,
    previews: &mut [Presented; DECKS],
    bind_groups: &[Option<wgpu::BindGroup>; DECKS],
    encoder: &mut wgpu::CommandEncoder,
) {
    for (slot, pres) in previews.iter_mut().enumerate() {
        if pres.aimed {
            if let Some(bg) = &bind_groups[slot] {
                present.draw_with_bind_group(encoder, bg, &pres.target, pres.size);
            }
        }
    }
}

/// **Is anything making texels this frame?** — which is the whole of what
/// decides whether the loop asks for another frame.
///
/// **The rule, rather than the expression: anything that makes texels this
/// frame keeps the loop awake, and the list is closed.** Everything the engine
/// draws into is read here — the picture, which is the one sink `compose` is
/// handed, and all [`DECKS`] preview cells, which [`monitor`] draws in that
/// frame's `finally` off each slot's own target — so a target added later and
/// not added to this is the same bug again, and it is the bug this file has
/// already shipped once: `live` was the picture alone, so folding the picture
/// away left deck A auditioning under it while the loop stopped asking for
/// frames. It fails in whichever direction the
/// mistake is made — a window that goes on drawing what nobody asked for, or a
/// panel that keeps changing while the loop sleeps.
///
/// A function rather than an expression in the frame path for the reason
/// [`Readout::pointer`] is a method: `window_event` cannot be called from a
/// test, so the part worth asserting is lifted out to where a test can reach
/// it — see `anything_that_makes_texels_keeps_the_loop_awake`.
pub(crate) fn live(view: &View) -> bool {
    // **The projector is the third thing on the list**, and leaving it off is
    // the bug this function's own paragraph describes, one output along: with
    // the picture folded away and a projector on, the loop would stop asking
    // for frames while a window on another display went on showing whatever
    // was last presented into it — the show stopped, with nothing said.
    view.picture.is_some() || view.previews.iter().any(Option::is_some) || view.projector
}

/// **What the transport row reads this frame**, out of the two things in this
/// file that know: the deck's oscillator, and what the last frame cost.
///
/// `Transport` here is `karakuri_console::view::Transport` — the console's row
/// of readouts — and not `karakuri_engine::transport::Transport`, which is a
/// slot's sync mode and is a different thing with the same word on it. This
/// takes a `&Deck` because it is on the side of the seam that is allowed one;
/// what crosses into the console is six numbers (ADR-0156).
///
/// # Where each number comes from, and that nothing is measured twice
///
/// - **The tempo, the position and the grid.** `Deck::signals` is the
///   session's one oscillator — the same one every binding reads — and
///   `Oscillator::bpm` and `Oscillator::beats` are its tempo and its musical
///   position. `beats` is unbounded and monotone, so which dot is lit and
///   which bar it is are arithmetic on it and the console does that
///   arithmetic. The deck advances it inside `render`, by the frame's own
///   measured step count ([`App::clock`], and not the fixed one a frame
///   carried until 2026-09-08), so this reads the position as of the end of
///   the last frame.
/// - **The frame's cost.** `Cost::whole` — the same three fields the reading
///   sums under *"the whole frame is a median"*, for the frame just drawn.
///   Nothing is timed twice: `Costs::push` kept the last `Cost` and this
///   divides nothing.
/// - **The rate.** `Costs::rate_now`, which is the reading's own `rate`
///   asked before its deadline rather than at it.
/// - **How many beats a bar has.** `karakuri_signal::oscillator::BEATS_PER_BAR`, which is
///   where the deck's own grid gets it, and which says of itself that it is
///   provisional until the IR format carries a time signature. Asked rather
///   than transcribed, so that the day it stops being 4 the beat grid stops
///   being four dots.
///
/// `None` before the first frame has been drawn, which is one frame of a run:
/// there is no frame cost yet, and a row that made one up would be inventing
/// exactly the reading this whole seam exists to refuse.
///
/// **The rate is `None` unless something is live**, and that is not caution
/// either. With nothing making texels this loop stops asking for frames, so
/// the last rate it measured would sit in the row describing a window that has
/// stopped drawing — the one number here that goes stale by standing still.
/// The frame time beside it does not: the last frame did cost that, whenever
/// it was.
pub(crate) fn transport(
    deck: &Deck,
    costs: &Costs,
    budget_ms: Option<f32>,
    live: bool,
    // **What the last write did**, which is the one thing in this row that is
    // not read off the deck here: a verdict exists once, in the drain
    // `staging` makes, so it is remembered in `Readout::health` and handed
    // over rather than asked for.
    health: Option<view::Stage>,
    // **Whether a session is being recorded**, which is the second thing in
    // this row that is not read off the deck: the recorder is this window's
    // and lives on [`App::recording`], so it is handed over rather than asked
    // for. It is always a value on this side — a program that holds a store
    // can always answer *am I recording* — and the console's `None` is for a
    // console nobody told.
    rec: view::Rec,
) -> Option<view::Transport> {
    let last = costs.last?;
    let grid = deck.signals().oscillator();
    Some(view::Transport {
        bpm: grid.bpm(),
        beats: grid.beats(),
        beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
        fps: live
            .then(|| costs.rate_now())
            .flatten()
            .map(|rate| rate as f32),
        frame_ms: ms(last.whole()) as f32,
        budget_ms,
        health,
        rec: Some(rec),
    })
}

/// **What the store holds, summarised**: every Set in it, most recent first,
/// with what each one is made of.
///
/// # It asks `setfile::summarise` and not `Store::list_sets`, and that is the
/// whole of what the filter fields needed
///
/// This used to be `Store::list_sets().map(|entry| entry.id)` — a directory
/// read and a name per file — and `view::LibraryBay` said beside the two
/// undrawn filter fields that *"nothing in the store answers it: `list_sets`
/// reads names off a directory and no index anywhere says what a Set holds"*.
/// The second half of that was already wrong:
/// [`karakuri_environment::setfile::summarise`] reads what each Set holds, node
/// by node, and `karakuri-environment`'s MCP `list_sets` has been narrowing on
/// it. So the reading moves here, one Set file per row on top of the directory
/// read, and [`narrows`] is the same retain the tool does.
///
/// **Most recent first, which `Store::list_sets` is not.**
/// `docs/manual/operations.html`'s row says the listing is most recent first
/// and the MCP tool sorts for it; the bay was showing ascending id, so the
/// panel and the tool answered one operation two ways. The tie-break is the id
/// and it is not decoration — two Sets written inside one tick of a coarse
/// filesystem clock carry the same mtime, and a sort whose keys tie leaves
/// whatever order the entries arrived in.
///
/// # Read once, and by the side of the seam that may read a disk
///
/// Called from `resumed`, before the first frame. A listing is a directory
/// read and a frame path does not do those
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)),
/// and nothing in `karakuri-console`'s `src/` could do it anyway: opening a
/// store is `karakuri-store`'s and the crate depends on neither it nor the
/// engine (ADR-0156). What crosses into the console is a list of names.
///
/// The cost of reading it once is that a Set saved while this window is up
/// does not appear in the bay until the next run. That is this program's
/// limitation and not the console's — the field is rewritable per frame like
/// every other one — and closing it wants a reason to re-read rather than a
/// timer, which is a decision and not this pass's.
///
/// # A missing store is listed as nothing, and is not created
///
/// [`karakuri_store::store::Store::open`] *"establishes the store layout under
/// `root`, creating any directories that do not exist yet"*, which is the
/// right thing for a program that is about to write one and the wrong thing
/// for one that only wants to read. A program that listed a library by first
/// making one would change the directory it was run in, so the root is
/// required to be there already.
///
/// Either way the answer is a list, and an empty one is a bay with nothing in
/// it — which is what `view::library` draws for it, and is honest: a store
/// this run could not read holds nothing it can name.
pub(crate) fn procedures(root: &std::path::Path) -> Vec<ListedProcedure> {
    if !root.is_dir() {
        // Said once by `library` beside it on the same press, so this one is
        // quiet: two lines about one missing store would be one fact said
        // twice.
        return Vec::new();
    }
    let store = match Store::open(root) {
        Ok(store) => store,
        Err(e) => {
            println!("library: {} could not be opened: {e}", root.display());
            return Vec::new();
        }
    };
    let listed = match store.list_procedures() {
        Ok(listed) => listed,
        Err(e) => {
            // Said rather than swallowed, for [`library`]'s reason: a tier
            // that is empty because a directory could not be read looks
            // exactly like a tier nobody has kept into.
            println!(
                "library: {}/{} could not be listed: {e}",
                root.display(),
                Store::PROCEDURES
            );
            return Vec::new();
        }
    };
    let mut out: Vec<ListedProcedure> = listed
        .into_iter()
        .filter_map(|entry| {
            // **One small read per row, and it compiles nothing** — the badge
            // is the file's `kind` line, which `history::declared_kind` scans
            // (ADR-0338, P-0091). A file that will not read is dropped rather
            // than drawn: it is a fact about this disk at this instant and
            // there is nothing to put in a row.
            let source = store.read_procedure(&entry.name).ok()?;
            Some(ListedProcedure {
                name: entry.name,
                written: entry.written,
                kind: karakuri_environment::history::declared_kind(&source).and_then(kind_of),
            })
        })
        .collect();
    out.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.name.cmp(&b.name)));
    out
}

/// **What a Set's row is**: the layers its own `slot` records fill, once each
/// and in the file's own order.
///
/// **Read off the summary the listing already has**, which is why a badge costs
/// nothing beyond what `all` was already paying: `setfile::summarise` reads a
/// line per node to answer *what is in my library*, and the layer is one of the
/// fields it already carries (P-0091, ADR-0338).
///
/// **Once each**, because a badge says *this Set fills that layer* and a Set of
/// three renderers fills L4 once for the purpose of reading a row.
pub(crate) fn set_row(set: &setfile::SetSummary) -> RowKind {
    let mut badges: Vec<karakuri_operation::Layer> = Vec::new();
    for node in &set.nodes {
        let layer = asked(node.layer);
        if !badges.contains(&layer) {
            badges.push(layer);
        }
    }
    RowKind {
        badges,
        procedure: false,
    }
}

/// **What a kept procedure's row is**: its one declared kind, and that it is a
/// procedure. A file that declares none draws no badge.
pub(crate) fn kept_row(kept: &ListedProcedure) -> RowKind {
    RowKind {
        badges: kept.kind.into_iter().collect(),
        procedure: true,
    }
}

/// [`kept_row`] one tier along, for a procedure that ships.
pub(crate) fn shipped_row(shipped: &karakuri_environment::places::PresetProcedure) -> RowKind {
    RowKind {
        badges: shipped.kind.and_then(kind_of).into_iter().collect(),
        procedure: true,
    }
}

/// **Does this procedure pass the kind row?**
///
/// `LibraryKinds::shows_layer` answers it for a file that declares a kind, and
/// this adds the one case that value cannot carry: **a `.kir` with no `kind`
/// line shows only while nothing is on**. It is a row of the library either way
/// — dropping it would answer *what have I kept* with a file missing — and a
/// chip that named it would be a chip claiming it is of a kind nobody wrote.
pub(crate) fn shows_kept(
    kinds: karakuri_operation::LibraryKinds,
    kind: Option<karakuri_operation::Layer>,
) -> bool {
    match kind {
        Some(layer) => kinds.shows_layer(layer),
        None => !kinds.narrowing(),
    }
}

/// **What the presets root ships that can go over a layer**, as its rows —
/// [`presets_listing`]'s shape one extension along and with its failures.
pub(crate) fn presets_procedures(
    presets: Option<&karakuri_environment::places::Presets>,
) -> Vec<karakuri_environment::places::PresetProcedure> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_procedures() {
        Ok(procedures) => procedures,
        // **Said out loud and then empty**, which is [`presets_listing`]'s own
        // answer beside it: a tier that is empty because a directory could not
        // be read looks exactly like one that ships nothing.
        Err(why) => {
            println!("library: {why}");
            Vec::new()
        }
    }
}

/// **One procedure the operator has kept**, as [`procedures`] found it: the
/// name its row is drawn under, when it was written, and the layer it declares.
///
/// **`ListedProcedure` and not `Kept`**, which is taken: [`Kept`] is what a
/// press on the Inspector's `keep` capsule *files*, and this is a row of the
/// listing that files show up in. One is an act and the other is a listing, and
/// a name over both would be a name meaning two things.
///
/// **`kind` is an `Option` and a `None` is a row**, which is
/// `places::PresetProcedure::kind`'s own note one tier along: a `.kir` with no
/// `kind` line is a file somebody kept, and dropping it from the listing would
/// answer *what have I kept* with a file missing and nothing said. It draws no
/// badge, and a load off it is refused by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListedProcedure {
    pub(crate) name: String,
    pub(crate) written: std::time::SystemTime,
    pub(crate) kind: Option<karakuri_operation::Layer>,
}

pub(crate) fn library(root: &std::path::Path) -> Vec<setfile::SetSummary> {
    if !root.is_dir() {
        println!(
            "library: no store at {}, so the bay lists nothing",
            root.display()
        );
        return Vec::new();
    }
    let sets = Store::open(root).and_then(|store| setfile::summarise(&store));
    match sets {
        Ok(mut sets) => {
            sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
            sets
        }
        Err(e) => {
            // Said rather than swallowed, for the reason every other failure
            // in this file is said: a bay that is empty because the store
            // could not be read looks exactly like a bay that is empty
            // because the store is.
            println!("library: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// **Which Sets this store has starred**, as their ids — the other half of
/// what the Library bay lists, and the half `my sets` *is* (ADR-0299).
///
/// # It is [`library`]'s shape one file along, and its failures are the same
///
/// A store that is not there holds no stars, a store that will not open is
/// **said out loud** rather than answered with silence, and either way the
/// answer is a set — because a scope that is empty because a file could not be
/// read looks exactly like one that is empty. The one difference from
/// [`library`] is that a missing `favourites.json` is not a failure at all:
/// `Store::favourites` answers an empty set for it, which is a store nobody
/// has starred in.
///
/// **Nothing is pruned against the listing here.** A mark whose Set a hand
/// removed from `sets/` stays in the file — that is `Store::favourites`' own
/// rule and ADR-0299's — and what makes it harmless is that [`listing`] takes
/// the *intersection* with what the store holds, so an id naming no Set draws
/// no row and can still have its star taken off.
///
/// # Off the frame, on the press that builds a listing
///
/// One file read, beside the directory read [`library`] already does
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)):
/// at startup and on every press that re-lists, which is a scope, a filter and
/// a star. `karakuri-console` reaches no disk at all (ADR-0156), so what
/// crosses the seam is the set of ids.
pub(crate) fn favourites(root: &std::path::Path) -> std::collections::BTreeSet<String> {
    if !root.is_dir() {
        return std::collections::BTreeSet::new();
    }
    match Store::open(root).and_then(|store| store.favourites()) {
        Ok(starred) => starred,
        Err(e) => {
            println!(
                "library: {} keeps no readable favourites: {e}",
                root.display()
            );
            std::collections::BTreeSet::new()
        }
    }
}

/// **What one Set declares, read off the cards the store keeps beside its
/// artifacts** — the answer to
/// [`Operation::ReadSet`](karakuri_operation::Operation::ReadSet), in the shape
/// the Library bay draws it in.
///
/// # It is the same reading the MCP tool gives a model, through the same path
///
/// `karakuri_mcp::read_set` renders this for a model, and what it
/// reads is the Set file's `slot` records and each artifact's metadata card —
/// `Store::read_set`, then `Store::read_meta` per node, then the `param_decl`,
/// `capacity_decl` and `emit` records on it. **This walks the same records**
/// rather than a second source: a panel and a model that disagreed about what
/// a Set declares would be two answers to one question. What differs is the
/// rendering, and it differs because the destinations do: a model is handed
/// prose it reads in a context window and a bay is handed one line per control
/// in a column eight characters wide.
///
/// **The prose renderer is not called, and could not be**: it returns one
/// `String` per Set with its blocks already laid out in sentences, and a
/// listing row wants the key and the range apart. Lifting a structured reading
/// into `karakuri-environment` so that both surfaces render one value is the
/// right shape and is a change to a crate this pass may not touch; what is
/// here is written against the same records in the same order so that the day
/// somebody does, this is what moves.
///
/// # The three blocks the row promises, and the fourth that is not here
///
/// *"Every knob with its range and default, the element count, the attributes
/// emitted — each read off the artifact's own card, so those three fetch no
/// source and compile nothing."* The three are each a record on a card.
/// **The element storage is deliberately absent**: the MCP
/// tool's `element_storage_block` calls `setfile::load`, which fetches every
/// source in the Set and runs `compile::check` over it, so a panel that drew
/// it would be paying exactly the cost that sentence says these three do not.
/// `docs/manual/console.html`'s note says the same and says where the figure
/// goes instead — *"A model asking over MCP gets it and pays for it; a press
/// in a library list is not the place to spend that"* — which the row settles
/// the same way.
///
/// # One control per key, and the range is the one every node agrees to
///
/// A Set publishes one control per *key* and not one per declaration
/// (`docs/ir-spec.md`, and `karakuri_engine::set::Set::published`), so a name
/// two nodes declare is one row over the part of the range both of them
/// accept. That intersection is `Set::declared_range`'s own arithmetic —
/// `lo.max(min)`, `hi.min(max)` — done here because **a built Set is what this
/// chip exists to be pressed before**: `published()` needs an engine and a
/// device, and a reading that took one would be the load it is meant to save.
///
/// The **default** is the first declarer's, and the order is the file's own —
/// the same order `read_set` prints its nodes in and the order
/// `Set::published` walks. A default is one number and two nodes may declare
/// two; the alternative is drawing neither, which is a blank row and is the
/// one thing the mock's own tip refuses.
pub(crate) fn declared(root: &std::path::Path, id: &str) -> Result<Reading, String> {
    let store = Store::open(root).map_err(|e| format!("the store at {}: {e}", root.display()))?;
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    let mut reading = Reading {
        id: id.to_owned(),
        ..Reading::default()
    };
    // The key's range so far and the first declarer's default, in the order
    // the keys first appeared — a `Vec` and not a map for `Set::published`'s
    // own reason: the order is what the author wrote, and a hasher's order
    // would move between runs.
    let mut knobs: Vec<(String, [f32; 2], Option<f32>)> = Vec::new();
    let mut capacity: Option<[u32; 3]> = None;
    let mut emits: Vec<String> = Vec::new();
    for line in &lines {
        let Record::Slot { proc_hash, .. } = line.record() else {
            continue;
        };
        reading.nodes += 1;
        // **A node with no card is counted rather than skipped in silence.**
        // A card is derived rather than kept, so an artifact stored as bytes
        // has none — an ordinary state of a working store, which
        // `mcp::node_block` says at length — and what it costs this reading is
        // whatever that node declared. The foot is where that is said.
        let Ok(card) = store.read_meta(proc_hash) else {
            continue;
        };
        reading.described += 1;
        for entry in &card {
            match entry.record() {
                Record::ParamDecl {
                    key,
                    min,
                    max,
                    default,
                    ..
                } => match knobs.iter_mut().find(|(seen, ..)| seen == key) {
                    Some((_, range, _)) => {
                        range[0] = range[0].max(*min);
                        range[1] = range[1].min(*max);
                    }
                    None => knobs.push((key.clone(), [*min, *max], *default)),
                },
                Record::CapacityDecl { min, max, default } => {
                    capacity = Some(match capacity {
                        None => [*min, *max, *default],
                        Some([lo, hi, was]) => [lo.max(*min), hi.min(*max), was],
                    })
                }
                // **The union, in the order the nodes declare them.** A Set
                // over two geometries emits what both of them do, and the row
                // is *what a renderer drawn over it can consume* rather than
                // any one node's list.
                Record::Emit { attrs } => {
                    for attr in attrs {
                        if !emits.contains(attr) {
                            emits.push(attr.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    reading.knobs = knobs
        .into_iter()
        // **Spelled `view::Published` in full**, because
        // `karakuri_engine::set::Published` is in scope here under the same
        // name and they are the same idea two levels apart: one is what a
        // built Set publishes and this is what a file's cards say it will.
        .map(|(key, [min, max], default)| view::Published {
            key,
            range: spelled(
                &format!("{min}"),
                &format!("{max}"),
                default.map(|d| format!("{d}")),
            ),
        })
        .collect();
    reading.capacity = capacity.map(|[min, max, default]| {
        spelled(
            &format!("{min}"),
            &format!("{max}"),
            Some(format!("{default}")),
        )
    });
    reading.emits = (!emits.is_empty()).then(|| emits.join(", "));
    Ok(reading)
}

/// **A declared range and the default that applies until something turns it**,
/// in the shape the mock draws every line of a reading in: `0 – 8 · 2`.
///
/// **The three marks are the mock's own** — an en dash between the ends of the
/// range and a middle dot before the default — and both are in the face the
/// panel draws with, which is asserted rather than assumed
/// ([`the_marks_a_reading_is_spelled_with_are_in_the_face`]): the Library
/// bay's foot read `load → A` with the arrow typed as a U+2192 the default
/// face does not carry, and drew `load □ A` for a release. The arrow is drawn
/// rather than typed now, and is a label between two capsules (ADR-0305).
///
/// **A default that is not a literal is a word and not a blank.** The card's
/// `default` is absent where the declaration's expression is not a number this
/// build can state — never where there is none, since the `.kir` grammar makes
/// the expression mandatory — so what a blank would say here is false. `expr`
/// is what is drawn instead, in a column that has room for four characters.
pub(crate) fn spelled(min: &str, max: &str, default: Option<String>) -> String {
    format!(
        "{min} – {max} · {}",
        default.unwrap_or_else(|| "expr".to_owned())
    )
}

/// **The reading under the cursor, read and written into the view**, and the
/// sentence to print about it.
///
/// [`listing`]'s shape one control along, and it is here for that function's
/// reason: reading a Set file and the cards behind it is a disk read, this is
/// the side of the seam that owns the store, and `karakuri-console` takes none
/// of the three (ADR-0156). **On the press and never on a frame** (P-0091) —
/// which is the press on the `params` chip, and the key that moves the cursor
/// while a reading is open, because the reading follows the cursor.
pub(crate) fn read_reading(view: &mut View, store: &std::path::Path) -> String {
    // **The Sets, which is empty under `history`**: a reading is what a Set
    // declares and a row of that scope is a version, so the cursor has no
    // operand there — `view::View::sets`.
    let Some(id) = view.sets().get(view.cursor_row()).cloned() else {
        // A press with no row under the cursor asks nothing —
        // `LibraryBay::read` answers `None` for it — so this is reachable only
        // from a cursor that moved in a listing that went empty in between, or
        // from a reading left open while the bay was marked `history`: the
        // block is not drawn there (`View::opened` matches the row's name), and
        // it is put away here rather than left holding an answer about a Set
        // nobody can see.
        view.shut_reading();
        return String::from(match view.scope() {
            Some(scope) if !scope.lists_sets() => {
                "  read: `history` lists the versions of a Set rather than Sets, so there is \
                 nothing under the cursor for a reading to be about"
            }
            _ => "  read: this bay lists nothing, so there is no Set to read",
        });
    };
    match declared(store, &id) {
        Ok(reading) => {
            let line = format!(
                "  read: `{id}` declares {} over {}{}",
                reading.knobs_word(),
                reading.nodes_word(),
                match reading.nodes == reading.described {
                    true => String::new(),
                    false => format!(", {}", reading.cards_word()),
                }
            );
            view.read(reading);
            line
        }
        // **Said out loud and the block put away**, which is [`library`]'s
        // rule one bay up: a reading that failed to read and a Set that
        // declares nothing must not draw the same.
        Err(e) => {
            view.shut_reading();
            format!("  read: `{id}` could not be read — {e}")
        }
    }
}

/// **The reading follows the cursor, on whichever surface moved it.**
///
/// `karakuri-console/src/view.rs` states the rule on `View::reading_open`:
/// *"a move with one open is a read of the row it arrived at, and a move with
/// nothing open is a pointer moving"*. **Two surfaces move that cursor**, the
/// arrow keys through `View::walk` and a carry's press through
/// `View::point_at`, and both owe it the same read — this is the one place
/// that read is written, so there is one implementation of the rule for both
/// to call rather than two copies that could answer it differently.
///
/// `moved` is each caller's own answer to *did this press move the cursor*:
/// the arrow-key arm compares `View::cursor_row()` before and after the
/// press, and the carry's arm is `matches!(acted, Acted::Pointed)`. Neither
/// shape is repeated here, because *what counts as a move* is each surface's
/// own question and this function's only question is what to do once one
/// has happened.
///
/// # The defect this rule exists to prevent
///
/// Until ADR-0265, `Readout::took` discarded `View::point_at`'s `moved`. A
/// carry taken in hand while a reading was open on a **different** row moved
/// the cursor off it, `View::opened` answered `None` because the row under
/// the cursor was no longer the Set the reading was of, and the block
/// vanished with nothing on that route ever walking the cursor back — no
/// panic, no diagnostic, a reading that stopped being drawn. The arrow keys
/// never had the bug — they always re-read on `moved && reading_open()` — so
/// the two call sites already agreed before this function existed; what it
/// buys is that they cannot silently stop agreeing.
///
/// Returns the line to print rather than printing it, so a caller with
/// nothing to print — the ordinary case, a press with no reading open — pays
/// for no `println!` and a test can call this with no stdout to capture.
pub(crate) fn reread_if_open(
    moved: bool,
    view: &mut View,
    store: &std::path::Path,
) -> Option<String> {
    (moved && view.reading_open()).then(|| read_reading(view, store))
}

/// **The record's layer a vocabulary layer names.**
///
/// A third spelling of a list that already has two conversions in
/// `karakuri-environment` — `setfile::kind_of` and `mcp.rs`'s pair — and it is
/// here because both of those are private to that crate and neither is on its
/// way out. What crosses the seam is the summary, whose nodes carry a
/// `karakuri_store::record::Layer`, and what a filter carries is a
/// `karakuri_operation::Layer`; the comparison has to happen on one side.
///
/// **Exhaustive with no wildcard**, which is what makes it a table rather than
/// a guess: a sixth layer does not compile until somebody says which it is.
pub(crate) fn asked(layer: karakuri_store::record::Layer) -> karakuri_operation::Layer {
    use karakuri_operation::Layer as Asked;
    use karakuri_store::record::Layer as Written;
    match layer {
        Written::L1 => Asked::L1,
        Written::L2 => Asked::L2,
        Written::L3 => Asked::L3,
        Written::L4 => Asked::L4,
        Written::Field => Asked::Field,
        Written::L5 => Asked::L5,
    }
}

/// **The word a `kind` line spells a layer with, as the vocabulary's own
/// value** — `karakuri_environment::history::LAYERS`' six words read back.
///
/// `None` for a word that is not one of the six, which is a `.kir` declaring
/// no kind at all: `declared_kind` already answers `None` there, and this is
/// the same absence carried one step further rather than a second reading of
/// it.
///
/// **The wildcard is the risk here and it is why `LAYERS` is the list.** This
/// match answers a word rather than a value, so a sixth kind does *not* stop
/// the build — it falls to `None` and a `kind L5` file reads as one declaring
/// nothing. That is the failure `setfile::layer_from_ordinal`'s comment names
/// one layer earlier, arriving through the other door.
pub(crate) fn kind_of(word: &str) -> Option<karakuri_operation::Layer> {
    use karakuri_operation::Layer;
    match word {
        "L1" => Some(Layer::L1),
        "L2" => Some(Layer::L2),
        "L3" => Some(Layer::L3),
        "L4" => Some(Layer::L4),
        "Field" => Some(Layer::Field),
        "L5" => Some(Layer::L5),
        _ => None,
    }
}

/// **Which kinds the filter row is showing, as a sentence** — the words of the
/// chips that are on, or *every kind* where none of them is.
///
/// `karakuri_operation::LibraryKinds::narrowing` settles the reading of *none
/// on* once and this says it in the panel's own words: a row that hid the whole
/// listing would be a state an operator cannot see their way out of.
pub(crate) fn showing(kinds: karakuri_operation::LibraryKinds) -> String {
    if !kinds.narrowing() {
        return String::from("every kind");
    }
    let on: Vec<&str> = karakuri_console::view::KindChip::ALL
        .into_iter()
        .filter(|chip| chip.on(kinds))
        .map(|chip| chip.word())
        .collect();
    on.join(", ")
}

pub(crate) fn recorded(layer: karakuri_operation::Layer) -> karakuri_store::record::Layer {
    use karakuri_operation::Layer as Asked;
    use karakuri_store::record::Layer as Written;
    match layer {
        Asked::L1 => Written::L1,
        Asked::L2 => Written::L2,
        Asked::L3 => Written::L3,
        Asked::L4 => Written::L4,
        Asked::Field => Written::Field,
        Asked::L5 => Written::L5,
    }
}

/// **Does this Set pass the filter row?** — the same retain
/// `karakuri-environment`'s MCP `list_sets` applies, and deliberately so: one
/// operation narrowed two ways is two answers to *what does this store hold*.
///
/// **A Set matches, not a node.** With both filters given the question is
/// *which of the Sets that hold this also have something on that layer*, so
/// each half is answered against the whole Set — a Set whose `drift_shell` is a
/// geometry and whose deformation is called something else is exactly what that
/// question is looking for.
///
/// `holds` is matched case-insensitively against what each node is called,
/// because an operator who read `drift_shell` in one row and stepped to
/// `Drift_Shell` in another is not asking a different question. **The fields
/// step through the names as the store spells them**, so today the fold changes
/// no answer; it is here because the tool's does and the two must not come
/// apart the day either field takes letters.
pub(crate) fn narrows(
    set: &setfile::SetSummary,
    holds: Option<&str>,
    layer: Option<karakuri_operation::Layer>,
) -> bool {
    let held = holds.map(str::to_ascii_lowercase);
    held.as_ref().is_none_or(|holds| {
        set.nodes
            .iter()
            .any(|node| node.name.to_ascii_lowercase().contains(holds))
    }) && layer.is_none_or(|layer| {
        let want = recorded(layer);
        set.nodes.iter().any(|node| node.layer == want)
    })
}

/// **What the `holds` field can be stepped to**: every name a node in this
/// store's Sets carries, once each and in one order.
///
/// **The unnarrowed listing's**, which is `view::View::holds`' own instruction
/// and the reason this takes the summaries before [`narrows`] has been near
/// them: candidates read off a filtered listing shrink as the filter bites, and
/// a step would then wander somewhere it could not come back from.
///
/// **Sorted, and not in the order the Sets were written.** The candidates are a
/// list somebody steps through one press at a time, so the order has to hold
/// still while they do it — where the rows above are ordered by recency and
/// move whenever anything is saved. `BTreeSet` is the sort and the deduplication
/// in one pass.
pub(crate) fn holds_choices(sets: &[setfile::SetSummary]) -> Vec<String> {
    sets.iter()
        .flat_map(|set| set.nodes.iter().map(|node| node.name.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// **What the filter row is narrowing to, in words**, or `None` where it is
/// narrowing nothing.
///
/// The console draws the two values and this says what they mean, which is the
/// division every other readout on this panel makes: a field reads `L4` and a
/// line says the listing under it is the Sets that hold one.
///
/// **`layer` arrives already spelled**, out of `view::Filters::layer_word` —
/// the word the field itself is drawing. A `{:?}` here would print `Field`
/// where the field reads `FIELD`, which is a readout and a sentence about it
/// disagreeing in the one place a reader can see both.
pub(crate) fn narrowing(holds: Option<&str>, layer: Option<&str>) -> Option<String> {
    match (holds, layer) {
        (None, None) => None,
        (Some(holds), None) => Some(format!("holding a node called `{holds}`")),
        (None, Some(layer)) => Some(format!("holding a {layer} node")),
        (Some(holds), Some(layer)) => Some(format!(
            "holding both a node called `{holds}` and a {layer} node"
        )),
    }
}

/// **One row of a scope whose rows are files**: the word the bay draws and the
/// file behind it.
///
/// Two fields because the bay lists **names** and a take-in needs a **path**:
/// what crosses into the console is a `String` per row
/// (`view::View::library`), and what this program has to be able to find again
/// on the press is the file that row came off.
///
/// **Two scopes have rows of this kind** — `presets`, which is a told
/// directory (ADR-0230), and `folder`, which is one somebody dropped on this
/// window (ADR-0275). They are one type because a row of either is a Set file
/// that is not in this store yet and a press on it is the same two operations
/// (`docs/manual/operations.html`'s *Send a Set to somebody, and take one
/// in*): the difference between them is which directory was listed, which is
/// [`Taking`]'s.
pub(crate) struct FileRow {
    /// What the row reads, which is the file's own name without its
    /// extension. **Not read out of the file**: a listing that opened
    /// twenty-three files to draw twenty-three rows would be a directory read
    /// doing a file read's work, and the id a take-in files the Set under is
    /// the one *inside* the file anyway — read there, on the press, by
    /// [`taking_in`].
    pub(crate) id: String,
    pub(crate) path: std::path::PathBuf,
}

/// **Every Set the preset library offers**, which is the `.kset` files in the
/// root this run resolved.
///
/// # One call, and the reading is not this program's
///
/// The listing is `karakuri_environment::places`', beside the resolution that
/// answers *where* the presets are: what a `.kset` is and which directory
/// holds them is that module's business, and a second program wanting the same
/// list must not read the same directory a second way. So this is the one
/// place in this program that knows a preset library can be listed at all, and
/// it knows nothing about how — the shape of a row, the order they come in,
/// and what a name the layout does not claim does are all answered there.
///
/// # What it lists, and why not the `.kir` files beside them
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing* settles it: *"A directory of `.kir` files is a directory of parts,
/// and a library lists what you can put on a deck."* A `.kir` is one node's
/// source addressed by its content and nothing in the vocabulary takes one, so
/// the parts are not rows — they are what the rows *name*.
///
/// **A root with nothing in it is a library nobody has filled**, and it is not
/// a failure: the scope lists nothing and the sentence about it is
/// [`why_nothing`]'s. A directory that will not open is said out loud, for
/// [`library`]'s reason one scope along — a scope empty because a directory
/// could not be read looks exactly like one that is empty.
pub(crate) fn presets_listing(
    presets: Option<&karakuri_environment::places::Presets>,
) -> Vec<FileRow> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_sets() {
        Ok(sets) => sets
            .into_iter()
            .map(|set| FileRow {
                id: set.id,
                path: set.file,
            })
            .collect(),
        // **Said out loud and then empty**, which is the same shape the store
        // side takes one scope along: a root that will not open looks exactly
        // like a root nobody has filled, and the difference has to be spoken
        // or it is not there. The sentence is `places`' own — it names the
        // path and how that path was arrived at — so an operator who typed
        // `--presets` reads something different from one whose checkout moved.
        Err(why) => {
            println!("presets: {why}");
            Vec::new()
        }
    }
}

/// **Every Set a dropped folder holds**, which is the Set files directly in
/// the directory this bay was pointed at (ADR-0275).
///
/// # What it lists, and why both spellings
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing*: *"A Set file and a bundle are the same file … so the scope draws
/// one kind of row rather than two"*, and the authored form that names its
/// parts by relative path *"is a Set file, is one row, and is taken in by the
/// same operation."* So both suffixes are listed and neither is a second kind
/// of row — `Store::SET_FILE_SUFFIX` for the resolved form and
/// `setfile::AUTHORING_SUFFIX` for the authored one, borrowed from the modules
/// that own them rather than spelled here. A `.kir` is **not** listed: it is
/// one node's source, nothing in the vocabulary takes one, and *"a directory of
/// `.kir` files is a directory of parts"*.
///
/// **A directory that holds `night.kset` and `night.kbset` lists two rows
/// reading `night`**, and that is deliberate rather than got to by accident:
/// they are two files, each of which is a Set, and choosing between them here
/// would be this listing inventing a precedence between the two forms.
/// **Whichever of them a press means is the take-in's question and it is
/// answered by refusing**: [`Taking::file`] finds the row by the word that was
/// pressed, two files wear that word, and a press that took one of them would
/// be picking for the operator between two rows they cannot tell apart on
/// screen. See there, where the refusal names both files.
///
/// # Ascending, one directory deep, and the name is all that is read
///
/// `places::Presets::list_sets`' three rules, carried over for its reasons:
/// `read_dir` hands back no order at all, a name the layout does not claim is
/// skipped rather than repaired, and nothing here opens a file — a malformed
/// Set is a refusal at the moment it is taken in, where the operator can see
/// which row they pressed. It is **not** [`library`]'s most-recent-first
/// (ADR-0263): that order is a store's, where a Set's time is when the
/// operator wrote it, and a folder full of files somebody copied has mtimes
/// that are facts about this machine's disk.
///
/// # Where this belongs, and it is not here
///
/// **`karakuri_environment::places` is the right home**, beside
/// `Presets::list_sets`, which answers the same question about a directory
/// this run was told about rather than one it was handed: this is that
/// function with two suffixes and no `Found` behind it, and a second walk of a
/// directory of Sets is a second answer to *what is a Set file called*. It is
/// here because ADR-0275's owed work was this file's, and the reason is
/// written down rather than left to be inferred — `declared`'s own shape one
/// bay over.
pub(crate) fn folder_listing(dir: Option<&std::path::Path>) -> Vec<String> {
    folder_files(dir).into_iter().map(|row| row.id).collect()
}

/// **The same walk with the file names kept**, which is what a take-in needs:
/// the bay lists words and the press has to find the file the word came off
/// again ([`FileRow`]).
///
/// **One walk and not two**, which is why [`folder_listing`] is a `map` over
/// this rather than a second `read_dir`: a listing the bay drew and a listing
/// the press searched that disagreed would be a press acting on a row nobody
/// saw. It is [`presets_listing`]'s shape one scope along, and that function
/// answers `FileRow`s for the same reason.
pub(crate) fn folder_files(dir: Option<&std::path::Path>) -> Vec<FileRow> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // **Said out loud and then empty**, which is `presets_listing`'s shape
        // one scope along and for its reason: a folder that will not open
        // looks exactly like a folder holding no Sets, and the difference has
        // to be spoken or it is not there. A dropped directory can go between
        // the drop and a press an hour later — it is somebody else's
        // directory, which this program neither made nor writes.
        Err(why) => {
            println!("  folder: `{}` could not be listed: {why}", dir.display());
            return Vec::new();
        }
    };
    let mut out: Vec<(String, std::ffi::OsString)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        // Non-UTF-8 falls out of the listing with everything else the layout
        // does not claim — `Store::list_sets`' own rule, no lossy repair.
        let Some(id) = name.to_str().and_then(|name| {
            name.strip_suffix(Store::SET_FILE_SUFFIX)
                .or_else(|| name.strip_suffix(karakuri_environment::setfile::AUTHORING_SUFFIX))
        }) else {
            continue;
        };
        // A subdirectory named like a Set file belongs to whoever made it.
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        out.push((id.to_owned(), name));
    }
    out.sort();
    out.into_iter()
        .map(|(id, name)| FileRow {
            id,
            path: dir.join(name),
        })
        .collect()
}

/// **Why the scope that is marked lists nothing**, in the words that say which
/// kind of nothing it is — and among these four chips there are two kinds.
///
/// Two of them are empty as *data*: a store nobody has saved into and a preset
/// root nobody has filled are libraries with nothing in them, which
/// `console.html` says outright — *"An empty tier is a library nobody has
/// filled rather than something gone wrong."* Fill either and the rows appear
/// with nothing else changing.
///
/// **`my sets` is a third kind, and it is neither of those**: the store may
/// hold plenty and nothing be starred, which is a listing that is empty
/// because of an answer rather than because of an absence (ADR-0299). What to
/// do about it is press a star, and the sentence says so.
///
/// **The last of them is empty as *machinery*, and it is the one that
/// changed:**
///
/// - **A folder nobody has pointed anywhere is empty for want of a gesture**,
///   and that is the one of the four that changed on 2026-09-08. It used to be
///   empty for want of machinery — this said *"nothing here reads a folder
///   dropped on this window yet"* — and [`folder_dropped`] is that machinery.
///   `Operation::ListSets` still has nowhere to put a directory and should
///   not: both its fields narrow what a store already holds, and which store
///   is asked at all is [`listing`]'s own answer. **So this scope has two
///   sentences and `pointed` is which**: no folder has been dropped yet, or
///   one has and holds no Set file. The second is [`Scope::AllSets`]' kind of
///   nothing — a library nobody has filled — read in somebody else's
///   directory.
///
/// It is said out loud on the step and again on a press, because a scope that
/// went quiet and a scope that is empty are the same experience — which is the
/// rule every other refusal in this file is written to.
///
/// **The fifth is a third kind again, and it has two sentences of its own.**
/// `history` lists the versions of the Set the load pulldown's deck is
/// running, so it can be empty because that deck is running *no Set* — a run
/// launched on a pair somebody typed, whose versions are filed under none
/// (ADR-0276, ADR-0304) — or because the Set it is running has not been
/// edited yet. The first is the one worth spelling out: a listing narrowed to
/// a Set matches a `None` row not at all rather than matching every one of
/// them, so *nothing here* is the true answer and not a filter that misfired.
///
/// `pointed` is whether the bay has a directory at all and `running` is
/// whether the pulldown's deck names a Set; each is read by one arm only, and
/// the other four are the same sentence whatever this window has been dropped
/// on and whatever any deck is playing.
pub(crate) fn why_nothing(scope: Scope, pointed: bool, running: bool) -> &'static str {
    match scope {
        Scope::AllSets => {
            "this store holds no Sets yet, which is a library nobody has filled: `k` keeps \
             what a deck is playing, and loading a preset leaves one here too"
        }
        Scope::MySets => {
            "nothing in this store is starred — `my sets` is the starred subset of `all` \
             and never the listing of it, so press the star at the left of a row under \
             `all` and that Set appears here"
        }
        Scope::Presets => {
            "this run found no preset library, or the one it found holds no `.kset` file — \
             `--presets DIR` is what names one, and a directory of `.kir` parts is not a \
             library"
        }
        Scope::Folder if !pointed => {
            "no folder has been dropped on this window yet — drag one off the desktop and \
             let go of it anywhere over this window, and this scope lists the Sets in it"
        }
        Scope::Folder => {
            "the folder this bay is pointed at holds no Set file, which is a directory \
             nobody has put one in: a folder scope lists `.kbset` and `.kset` files, and a \
             directory of `.kir` parts is not a library"
        }
        Scope::History if !running => {
            "the deck the `load` pulldown names is playing a typed pair rather than a Set, so \
             its versions are filed under no Set at all and a listing narrowed to one matches \
             none of them — aim that pulldown at a deck you have loaded a Set onto, or load \
             one"
        }
        Scope::History => {
            "this Set has no versions yet — every write that compiles is kept, so edit one of \
             its nodes, or let a model write one, and the version it replaced is the first row \
             here"
        }
    }
}

/// **What a folder over this window reads in the `.path` row**, written into
/// the console for the pass that is about to draw it.
///
/// **The one thing about this bay that is a frame's business**, and it is
/// `egui`'s doing rather than a choice here: `RawInput::take` *clones*
/// `hovered_files` where it *moves* `dropped_files`, so a drag over the window
/// is a fact about every pass while it lasts and there is no event to hang it
/// off. Nothing is asked of the file system for it — whether the path is a
/// folder is the drop's question (P-0091, ADR-0275) — and nothing is
/// allocated on a pass where the answer has not changed.
///
/// **More than one path over the window reads as none.** The row says *"the
/// path a release would set"*, a release sets nothing where two arrived
/// (ADR-0275), and drawing the first of them would be this row picking one out
/// of a list the desktop happened to build — which is the choice the refusal
/// below exists to refuse.
pub(crate) fn folder_over(view: &mut View, hovering: &[karakuri_console::egui::HoveredFile]) {
    let over = match hovering {
        [one] => one.path.as_deref(),
        _ => None,
    };
    // **Compared before it is written**, so a drag held still over the window
    // costs nothing per pass. `to_string_lossy` borrows for a path that is
    // UTF-8, which every path drawn here is in practice, and it is the same
    // conversion `Path::display` makes — so what is compared is what would be
    // drawn rather than an approximation of it.
    let same = match (over, view.incoming.as_deref()) {
        (None, None) => true,
        (Some(over), Some(shown)) => *over.to_string_lossy() == *shown,
        _ => false,
    };
    if !same {
        view.incoming = over.map(|path| path.to_string_lossy().into_owned());
    }
}

/// **A folder let go on this window**, which is how the `folder` scope is
/// given a directory — and the three answers ADR-0275 settles, in the words
/// that record and `console.html` write.
///
/// # It is done here, on the pass the drop arrives on, and nothing is put by
///
/// `dropped_files` is visible for exactly one pass and then gone
/// (`RawInput::take` moves it), so the release does the whole thing rather
/// than asking a question: it sets the directory **and** marks the `folder`
/// chip, and the listing under it is the next thing drawn. A bay that had put
/// the path aside and waited for the chip to be pressed would be waiting on an
/// operator who has already made the gesture, holding a path nothing will hand
/// it a second time.
///
/// **So the file system is asked here, on a frame**, which is the one place
/// this program does that and it is P-0091's rule rather than an exception to
/// it: what is asked once is asked once, and a drop is one act. A drop is also
/// the only moment the question can be asked at all — the event carries a path
/// and nothing else, and *"a file and a directory are indistinguishable at the
/// event"* (ADR-0275).
///
/// # The two refusals, and each is a policy the plumbing does not answer
///
/// **A path that is not a directory is refused, naming what was dropped.** A
/// file is not read as the folder it sits in — that would point this bay at a
/// directory nobody pointed at, which is the mistake the carry one bay over
/// refuses when it declines to snap a drop mark to the nearest strip
/// (ADR-0273) — and a `.kbset` is not taken in where it fell, because taking a
/// Set in is a press on a row of a listing and a file landing on this window
/// has no row under it.
///
/// **More than one path is refused, and all of them are.** A multi-item drag
/// arrives whole, so three folders let go together are three entries in one
/// pass and not three drops: there is no first to act on and a rest to ignore,
/// nothing says which was aimed at, and the order is the desktop's rather than
/// the operator's. The bay keeps the directory it had and the refusal counts
/// what arrived.
///
/// **Both name what arrived and what to do instead** (P-0083), and both say
/// where this library is still pointed — because a refusal that left an
/// operator wondering whether the bay had moved anyway is a refusal that costs
/// a second gesture to read.
///
/// `None` where nothing was dropped, which is every pass but one.
pub(crate) fn folder_dropped(
    view: &mut View,
    folder: &mut Option<std::path::PathBuf>,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    dropped: &[&std::path::Path],
) -> Option<String> {
    // **Where the bay is still pointed**, read before anything moves, because
    // both refusals say it and the accept below replaces it.
    let kept = match folder.as_deref() {
        Some(at) => format!("This library is still pointed at `{}`.", at.display()),
        None => String::from("This library is still pointed nowhere."),
    };
    let one = match dropped {
        [] => return None,
        [one] => *one,
        many => {
            return Some(format!(
                "  folder: {} paths were let go together and none of them was taken — a drop \
                 tells this window a path and never a place, so nothing says which of them was \
                 aimed at and the order is the desktop's rather than yours. Let go of one \
                 folder on its own. {kept}",
                many.len()
            ))
        }
    };
    // **Asked once, here.** `is_dir` would answer `false` for a path that
    // cannot be examined at all, which is a different thing and is said as one.
    match std::fs::metadata(one) {
        Err(why) => Some(format!(
            "  folder: `{}` could not be examined ({why}), so whether it is a folder is not \
             known and nothing was taken. {kept}",
            one.display()
        )),
        Ok(what) if !what.is_dir() => Some(match set_file(one) {
            true => format!(
                "  folder: `{}` is a Set file and what this takes is a folder — a Set is taken \
                 in by pressing its row in a listing, and a file let go on this window has no \
                 row under it. Let go of the folder that holds it and press the row. {kept}",
                one.display()
            ),
            false => format!(
                "  folder: `{}` is not a folder and what this takes is one — this bay is \
                 pointed at a directory and lists the Sets in it, so let go of the folder that \
                 holds it rather than the file itself. {kept}",
                one.display()
            ),
        }),
        Ok(_) => {
            *folder = Some(one.to_path_buf());
            // **The line the bay draws is spelled here**, which is
            // `View::library`'s seam one row up: this side reads the disk and
            // the panel is handed what to draw (ADR-0156).
            view.folder = Some(one.display().to_string());
            // **And the chip is marked in the same act**, which is the whole
            // of *the drop is recorded on the frame it is seen on*: the
            // directory and the mark are one gesture's outcome, and a bay
            // pointed at a folder it is not showing would be waiting for a
            // press nobody owes it.
            view.select_scope(Scope::Folder);
            // **Asked of the mark rather than of the call**, because
            // `select_scope` answers *whether it moved* and a second drop
            // while `folder` is already marked moves nothing: the sentence is
            // about where the mark **is**. `false` here is a console handed no
            // `folder` chip, which cannot mark one — the row is still drawn,
            // since it is where a send's save dialog opens whichever scope is
            // marked (ADR-0311), and the listing under it is whatever scope this
            // console does have.
            let marked = match view.scope() == Some(Scope::Folder) {
                true => "the `folder` chip is marked",
                false => "this console draws no `folder` chip, so nothing is marked",
            };
            // **`None`, and it cannot be anything else here**: this drop has
            // just marked the `folder` chip, so the listing being re-asked is
            // a directory's and never a history's, and a Set id handed in
            // would be a value nothing reads.
            let said = listing(view, store, presets, folder.as_deref(), None);
            // **And the cursor goes back to the top of a listing it has never
            // seen**, which is `View::select_scope`'s own rule reached the
            // other way: that method resets the cursor when the *mark* moves,
            // and a second drop while `folder` is already marked moves the
            // listing without moving the mark. Left where it was it would
            // point at the fifteenth row of a directory of three — a Set
            // nobody chose, sitting under a pill that says a press will load
            // it.
            view.point_at(0);
            Some(format!(
                "  folder: this library is pointed at `{}` — {marked} and the listing under it \
                 is what that directory holds\n{said}",
                one.display(),
            ))
        }
    }
}

/// **Whether a name is one a Set file wears**, which is the two suffixes
/// [`folder_listing`] lists and is asked here for one reason: an operator who
/// let go of a `.kbset` on this window was trying to take a Set in, and the
/// refusal owes them the press that does it (P-0083).
///
/// It is a **name** and not a reading: nothing is opened, exactly as nothing
/// is opened to draw a row.
pub(crate) fn set_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(Store::SET_FILE_SUFFIX)
                || name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX)
        })
}

/// **The rows the Library bay lists for the scope that is marked**, written
/// into the view, and the sentence to print about it.
///
/// # One function, and it is what a scope *is* on this program's side
///
/// The console draws a row of chips and marks one of them; **which listing
/// belongs under that mark is this side's answer**, because every one of the
/// four is something outside this process — a store, a told directory, a
/// filter over the first, a directory somebody names during the run — and
/// `karakuri-console` takes none of them (ADR-0156). So the seam is a `Vec` of
/// names, and this is the one place it is filled.
///
/// **On the press that changed the scope and at startup, never on a frame.** A
/// listing is a directory read (P-0091), which is the same rule [`library`]
/// states one scope down and the reason this is not called from the frame
/// handler.
///
/// All five answer with rows now, and each of the five can still answer with
/// none — [`why_nothing`] is where the sentences are, and it is one
/// function so that a scope which stops being empty stops being empty in one
/// place. **Three of them depend on something that happened during the run**:
/// `folder` is `None` until somebody drops a directory on this window
/// ([`folder_dropped`]), `my sets` is empty until somebody presses a star
/// ([`favourite`]), and `history` is empty until the deck the load pulldown
/// names is running a Set that has been edited.
///
/// # `history` is the one scope that is not a directory of Sets
///
/// Its rows are the versions of **one Set** — the one the load pulldown's
/// deck is running, which `running` carries — and they come off
/// `karakuri_environment::history::list`, most recent first, in that
/// function's own order rather than in one applied here (ADR-0263's argument
/// on a different listing).
///
/// **The narrowing is a Set and never a deck**, which is why `running` is an
/// id rather than a slot: two decks playing one Set have one history between
/// them, and a version is filed under the Set the slot was running
/// (ADR-0304). **A `None` row matches no Set** rather than matching every one
/// of them — a version written where there was no Set is a version of
/// nothing, and treating it as a wildcard would put another run's edits under
/// whatever Set happens to be loaded now (ADR-0276's own consequence).
///
/// **The cap is on the walk and not on the Set.** [`HISTORY_MOST`] rows are
/// asked for and the narrowing happens after, so a store whose day directories
/// hold several Sets' versions lists fewer of each; `Listing::stopped_short`
/// is what says the walk stopped with days unread, and it is said out loud
/// beside the count rather than left for the foot's `n of m` to imply.
pub(crate) fn listing(
    view: &mut View,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    folder: Option<&std::path::Path>,
    running: Option<&str>,
) -> String {
    let Some(scope) = view.scope() else {
        return String::from(
            "  library: this console was handed no scopes, so there is no library to list",
        );
    };
    // **The store's own listing is what the filter row narrows**, and that is
    // `Operation::ListSets`'s own scope rather than a shortcut here: the row is
    // *List what the **store** holds*, which is `all` — and `my sets` is that
    // same listing starred (ADR-0299), so both are narrowed by the same retain
    // and neither is a second reading. The other two listings are not the
    // store: `presets` is a told directory of files and `folder` is somebody
    // else's. So the candidates are emptied for them, which is what makes
    // `View::filters` read both fields as unset there rather than the bay
    // hiding rows under a filter it is not applying. The filter comes back with
    // the scope, because the position it is kept as is still there.
    let held = match scope {
        Scope::AllSets | Scope::MySets => library(store),
        _ => Vec::new(),
    };
    // **What the walk found, and what it did not.** Read here rather than
    // inside the arm below so that the sentence about it is written where
    // every other sentence about this listing is; `None` for every scope that
    // is not a history, which is every scope that is a directory of Sets.
    let walked = match scope {
        Scope::History => Some(walked(store, running)),
        _ => None,
    };
    view.holds = holds_choices(&held);
    // **The marks, beside the listing they are a subset of.** They are read on
    // every one of these presses rather than once for the run, because a star
    // is a press that changes them and this is the one place the bay is told
    // what the store says — see [`favourites`], and `view::View::starred`.
    //
    // **Read whichever scope is marked**, because the star is drawn on every
    // row of every listing: a `presets` row is a file this store does not hold
    // and its star is hollow, which is what `Store::set_favourite` refusing an
    // id `sets/` does not hold says at the other end.
    view.starred = favourites(store);
    // **Copied out rather than borrowed across the write below**: `filters`
    // borrows `View::holds`, and the listing is written into the same `View`.
    // **`layer` is `None` and no field sets it any more**, which is ADR-0338:
    // the `layer…` field is retired for the six kind chips, and
    // `Operation::ListSets` keeps the field for `list_sets` and `--list-sets`,
    // where *which Sets hold a node on this layer* now lives. It is kept in the
    // narrowing below rather than deleted from it because that predicate is
    // what the MCP tool applies too.
    let (holds, layer) = {
        let at = view.filters();
        (at.holds.map(str::to_owned), None)
    };
    // **What the operator has kept, and what ships**, which are the two tiers a
    // procedure is a row of (ADR-0227's shape a fourth time, ADR-0338). Read
    // here beside the Sets and on the same press, because the two halves of a
    // scope's listing are one answer: `all` gains `<store>/procedures/` and
    // `presets` gains the presets root's `.kir` files, and no other scope
    // gains either — `my sets` is the starred subset of the Sets and a star is
    // refused on anything else, and a `folder` row is a *take*, which nothing
    // does with a bare `.kir`.
    let kept = match scope {
        Scope::AllSets => procedures(store),
        _ => Vec::new(),
    };
    let shipped = match scope {
        Scope::Presets => presets_procedures(presets),
        _ => Vec::new(),
    };
    let kinds = view.filters().kinds;
    // **One listing of rows and not two lists side by side**, which is
    // ADR-0338's *a procedure is a row of the same list*: the names and what
    // each row is are built together here and split into the two fields the
    // console reads, so a badge can never describe the row above the one it is
    // drawn on.
    let rows: Vec<(String, RowKind)> = match scope {
        Scope::AllSets => {
            let mut rows: Vec<(String, RowKind, std::time::SystemTime)> = held
                .iter()
                .filter(|set| narrows(set, holds.as_deref(), layer))
                .filter(|_| kinds.shows_sets())
                .map(|set| (set.id.clone(), set_row(set), set.written))
                .chain(
                    kept.iter()
                        .filter(|kept| shows_kept(kinds, kept.kind))
                        .map(|kept| (kept.name.clone(), kept_row(kept), kept.written)),
                )
                .collect();
            // **Most recent first, and the name breaks a tie**, which is
            // `library`'s own order applied to the merged list: two files
            // written inside one tick of a coarse clock tie, and a tied sort
            // is not an order.
            rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
            rows.into_iter().map(|(id, kind, _)| (id, kind)).collect()
        }
        // **The starred subset, and it is an intersection rather than a
        // second listing** (ADR-0299): the rows are the store's own, in the
        // store's own order, keeping only the ids the marks name. A mark whose
        // Set is gone draws no row, which is what makes a stale mark a line in
        // a file rather than a hazard — and it can still have its star taken
        // off, because `Store::set_favourite` refuses only the starring.
        //
        // **And no procedure is here**, which is a decision rather than an
        // omission: a star is refused on an id `sets/` does not hold, so there
        // is nothing to star on a procedure row and the starred subset of the
        // Sets is exactly what this chip is (ADR-0338).
        Scope::MySets => held
            .iter()
            .filter(|set| view.starred.contains(&set.id))
            .filter(|set| narrows(set, holds.as_deref(), layer))
            .filter(|_| kinds.shows_sets())
            .map(|set| (set.id.clone(), set_row(set)))
            .collect(),
        // **What ships, in two kinds of file and one list**: the `.kset` files
        // and the `.kir` files beside them, in name order because a shipped
        // file's mtime is when this machine checked it out.
        //
        // **A preset Set row carries no badge**, and the cost is stated where
        // it is paid (P-0091): `Presets::list_sets` opens no file at all, so
        // what layers one fills is not known without reading twenty-two Set
        // files on a press. A procedure's kind *is* known, because that listing
        // reads one small file per row for it.
        Scope::Presets => {
            let mut rows: Vec<(String, RowKind)> = presets_listing(presets)
                .into_iter()
                .filter(|_| kinds.shows_sets())
                .map(|preset| (preset.id, RowKind::default()))
                .chain(
                    shipped
                        .iter()
                        .filter(|shipped| shows_kept(kinds, shipped.kind.and_then(kind_of)))
                        .map(|shipped| (shipped.name.clone(), shipped_row(shipped))),
                )
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            rows
        }
        // **The directory a folder was dropped on this window to name**, and
        // it is the host's for [`presets_listing`]'s reason one arm up: a
        // listing is a directory read, and this side is the only side with a
        // disk (ADR-0156). Pointed nowhere it lists nothing and the sentence
        // about it is [`why_nothing`]'s.
        //
        // **A `.kir` in it is not a row**, and that note stands as written: a
        // folder row is a *take*, `--take-in` checks every inlined source
        // against the address its `slot` record names, and a loose `.kir`
        // names nothing (ADR-0338).
        Scope::Folder => folder_listing(folder)
            .into_iter()
            .filter(|_| kinds.shows_sets())
            .map(|id| (id, RowKind::default()))
            .collect(),
        // **The versions of the Set the load pulldown's deck is running**,
        // already narrowed and already in order — see [`walked`], and this
        // function's own head for why the narrowing is a Set rather than a
        // deck and why a `None` row matches nothing.
        //
        // **The kind chips do not narrow it**, because a version is not a Set
        // and not a procedure: it is one node's source at one moment, and the
        // six chips partition the rows of a *library*.
        Scope::History => walked
            .as_ref()
            .map(|found| found.rows.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row, RowKind::default()))
            .collect(),
    };
    view.library = rows.iter().map(|(name, _)| name.clone()).collect();
    view.kinds = rows.into_iter().map(|(_, kind)| kind).collect();
    // **Only where the filter was applied.** `layer` survives a scope change —
    // it is the console's own value and not a position in a listing — so a bay
    // reading `presets` under a set `layer` field would otherwise report a
    // narrowing that narrowed nothing. What says so out loud is the press:
    // `Readout::narrowed`.
    let narrowed = matches!(scope, Scope::AllSets | Scope::MySets)
        .then(|| narrowing(holds.as_deref(), None))
        .flatten();
    // **What the walk has to say beside the count**, and it is empty for every
    // other scope: a truncated listing that read as a whole one is the failure
    // `Listing::stopped_short` exists to prevent, and the foot's `n of m`
    // counts the rows this side handed over rather than the ones it did not
    // reach.
    let aside = walked.map(|found| found.said).unwrap_or_default();
    let line = match (view.library.len(), narrowed) {
        // **A filter that hid everything is a different nothing from an empty
        // library**, and it is the one kind `why_nothing` cannot name: the
        // store is not empty, and what to do about it is press a field rather
        // than save a Set. It is the sentence the MCP tool answers the same
        // case with, for the same reason.
        (0, Some(narrowed)) => format!(
            "  library: `{}` lists nothing — none of the {} Set{} here {narrowed}, so the \
             filter row is what to press rather than `k`",
            scope.name(),
            held.len(),
            match held.len() {
                1 => "",
                _ => "s",
            }
        ),
        (0, None) => format!(
            "  library: `{}` lists nothing — {}",
            scope.name(),
            why_nothing(scope, folder.is_some(), running.is_some())
        ),
        (listed, Some(narrowed)) => format!(
            "  library: `{}` lists {listed} of {} Set{}, {narrowed}",
            scope.name(),
            held.len(),
            match held.len() {
                1 => "",
                _ => "s",
            }
        ),
        (listed, None) => format!(
            "  library: `{}` lists {listed} {}",
            scope.name(),
            match (scope, listed) {
                (Scope::History, 1) => "version",
                (Scope::History, _) => "versions",
                (_, 1) => "Set",
                (_, _) => "Sets",
            }
        ),
    };
    format!("{line}{aside}")
}

/// **How many rows of the edit history one walk asks for.**
///
/// `karakuri_environment::history::list` takes the number from its caller and
/// has no default, because *"what a bay can afford to draw and what a model can
/// afford to be handed are different numbers"* (P-0090) — so this is the
/// panel's answer and nowhere else's.
///
/// **Larger than any bay can draw, and small enough that the walk stops after a
/// handful of days.** The Library bay's list is one row per
/// `view::size::LIB_ROW_H`, so a full-height bay on a tall display draws a few
/// tens of them; the cost of the walk is one `read_dir` per day directory
/// entered and no file opened at all, and it stops entering them once it has
/// this many. What lies past it is not counted — counting it is the cost the
/// cap exists not to pay — and `Listing::stopped_short` says the walk stopped,
/// which is the property that matters.
pub(crate) const HISTORY_MOST: usize = 200;

/// **The rows the `history` scope lists, and the sentence about how they were
/// found.**
///
/// Split out of [`listing`] because it is the one arm of that function that
/// has something to say beside the count: the walk is capped and it is capped
/// on *days opened* rather than on this Set's rows, so a listing that stopped
/// short has to say so or it reads as the whole history.
///
/// **A row is the name the store filed the version under**, less the Set id
/// every row here shares — see [`version_row`], which is also how a landing
/// finds the file again.
pub(crate) struct Walked {
    pub(crate) rows: Vec<String>,
    pub(crate) said: String,
}

pub(crate) fn walked(store: &std::path::Path, running: Option<&str>) -> Walked {
    // **No Set, no rows, and not an error.** The deck is playing a pair
    // somebody typed; its versions are filed under no Set, and `why_nothing`
    // is where that is said in words.
    let Some(id) = running else {
        return Walked {
            rows: Vec::new(),
            said: String::new(),
        };
    };
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        // Said rather than swallowed, and the scope lists nothing: a history
        // that would not open is a different fact from a Set with no versions,
        // and the two must not draw the same empty list in silence.
        Err(why) => {
            return Walked {
                rows: Vec::new(),
                said: format!("\n  history: {why} — so this scope lists nothing"),
            };
        }
    };
    let rows: Vec<String> = found
        .versions
        .iter()
        // **`Some(id)` and never `None`.** A version written where there was
        // no Set is a version of nothing, so it matches no Set rather than
        // every one of them (ADR-0276).
        .filter(|version| version.set.as_deref() == Some(id))
        .map(version_row)
        .collect();
    let mut said = String::new();
    if found.stopped_short {
        said.push_str(
            "\n  history: the walk stopped with days unread, so this is part of what is \
             there rather than all of it",
        );
    }
    if found.unclaimed > 0 {
        said.push_str(&format!(
            "\n  history: {} entr{} under `history/` that this layout does not claim {} \
             passed over",
            found.unclaimed,
            match found.unclaimed {
                1 => "y",
                _ => "ies",
            },
            match found.unclaimed {
                1 => "was",
                _ => "were",
            }
        ));
    }
    Walked { rows, said }
}

/// **One version, as a row of the Library bay's list and as the word a landing
/// names it by.**
///
/// It is the name [`karakuri_environment::history::Snapshots::record`] wrote,
/// less the `@<set>` every row of one walk shares and less the `.kir` — when,
/// which slot, which layer and index, and what the procedure called itself,
/// which is what `Version`'s own head says a row is for.
///
/// **One spelling, used twice.** The bay is handed this and hands it back at
/// the press, and [`restored`] rebuilds it per candidate to find the file
/// again — so the row an operator pressed and the version that is landed
/// cannot come apart, and no path crosses the seam. That is
/// `SetTransfer::Take`'s arrangement: the panel re-asks the listing and finds
/// the row by the word that was pressed.
///
/// **The index is spelled only when it is not the first**, which is `record`'s
/// own rule read back rather than a second one: a `_0` on every L4 of every
/// ordinary run is noise in the way of what a person is scanning for.
pub(crate) fn version_row(version: &karakuri_environment::history::Version) -> String {
    // **The spelling is the history module's**, since 2026-09-10: a model
    // walking the same history over MCP reads rows out of `walk_history` and
    // hands one back in `Revision::Picked`, so a second `format!` here would be
    // a second answer to *what is this row called* — and the two surfaces hand
    // the name to each other (`docs/adr/0342-…`).
    version.filed_as()
}

/// **What a take-in did**: the file it read, the id that file filed itself
/// under, and the sentence `setfile::unbundle` reported.
///
/// **Three fields because the press has three callers for them and each is a
/// different question.** The `said` is what the operator reads. The `id` is
/// what the load that follows names, and it is the file's own rather than the
/// row's word. The `file` is what the *operation* names —
/// `Operation::TransferSet`'s `SetTransfer::Take { file }` carries a path,
/// *"because a file is what the only existing route takes"* — so it is
/// returned rather than re-derived: the listing is asked once, on the press,
/// and asking it a second time to name what was already taken in would be two
/// answers to *which file was this* with a directory read between them.
#[derive(Debug)]
pub(crate) struct TakenIn {
    pub(crate) file: std::path::PathBuf,
    pub(crate) id: String,
    pub(crate) said: String,
}

/// **Which listing a take-in's row came off**, and it is the whole of the
/// difference between the two scopes that have rows of files.
///
/// # Two scopes, one row, one press
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, and a
/// folder row is the same row again. `console.html` says the folder side in as
/// many words — *"a folder row is a **take**"*, *"A row here is taken into the
/// store and then loaded, which is one press because taking it in is what
/// gives it a name"* — so the two differ in **which directory was listed** and
/// in nothing else. That is what this type is, and it is why [`taking_in`]
/// takes one rather than a presets root.
///
/// **The `folder` half is what landed on 2026-09-08.** It was refused out
/// loud until then — *"a folder row is a **take**, taking a Set in from a
/// folder is not built"* — because the scope had no directory to list, which
/// ADR-0275 gave it.
///
/// # A path is derived here and never spelled by a surface
///
/// `Operation::TransferSet`'s `SetTransfer::Take { file }` carries a path, and
/// the rule that admits it is that **every route that fills it derives it from
/// something the program itself produced**. Both arms obey it the same way:
/// the listing is asked *again* on the press and the row is found by the word
/// that was pressed ([`Taking::file`]), so what a surface handed over is a
/// word off a listing this program read and never a path.
pub(crate) enum Taking<'a> {
    /// The preset library this run resolved (ADR-0230) — a told directory,
    /// and the same one [`presets_listing`] draws the rows of.
    Presets(Option<&'a karakuri_environment::places::Presets>),
    /// The directory somebody dropped on this window (ADR-0275), and `None`
    /// for a bay that has been pointed nowhere.
    Folder(Option<&'a std::path::Path>),
}

impl Taking<'_> {
    /// The rows this listing holds, **asked again** rather than kept — see
    /// [`taking_in`], where that rule is argued.
    pub(crate) fn rows(&self) -> Vec<FileRow> {
        match self {
            Taking::Presets(presets) => presets_listing(*presets),
            Taking::Folder(dir) => folder_files(*dir),
        }
    }

    /// What the scope is called, for a refusal to name.
    pub(crate) fn scope(&self) -> &'static str {
        match self {
            Taking::Presets(_) => "the preset library",
            Taking::Folder(_) => "the folder this bay is pointed at",
        }
    }

    /// **The file behind the word that was pressed**, or a sentence saying why
    /// there is not one.
    ///
    /// # Two refusals, and the second is the one a folder brought
    ///
    /// **A word this listing no longer holds** is the row having gone between
    /// the listing and the press — a directory this program neither made nor
    /// writes, which is a folder's ordinary condition and a preset root's
    /// unusual one.
    ///
    /// **A word two files wear** is `folder_files`' own note arriving: a
    /// directory holding `night.kbset` and `night.kset` draws two rows reading
    /// `night`, and neither the listing nor the bay puts a precedence between
    /// the two forms. **So the press is refused and both file names go back**
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
    /// because taking one of them would be this program choosing between two
    /// rows an operator cannot tell apart on screen. A `presets` root can
    /// hold only `.kset` files, so this arm is a folder's in practice and is
    /// asked of both because the rule is the row's rather than the scope's.
    pub(crate) fn file(&self, row: &str) -> Result<std::path::PathBuf, String> {
        let mut found: Vec<std::path::PathBuf> = self
            .rows()
            .into_iter()
            .filter(|held| held.id == row)
            .map(|held| held.path)
            .collect();
        match found.len() {
            0 => Err(format!("{} has no `{row}` in it any more", self.scope())),
            1 => Ok(found.remove(0)),
            _ => Err(format!(
                "{} holds {} files called `{row}` — {} — and this row names a word rather \
                 than a file, so which of them you meant is not something the listing can \
                 say. Rename or move one of them and press again",
                self.scope(),
                found.len(),
                found
                    .iter()
                    .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
                    .collect::<Vec<_>>()
                    .join(" and ")
            )),
        }
    }
}

/// **A row of `presets` or of a `folder`, taken into this store**, and the id
/// it landed under.
///
/// # Taking it in is not a second operation, and it is what gives it a name
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, so
/// opening a preset **is** that row performed. `console.html` reaches it from
/// the other side: *"loading a preset is a packaging step, and a packaging step
/// writes into the store: `my sets` gains a row you did not make."* That is
/// what this does, and it is why a preset row is one press rather than two —
/// the take-in is what gives the Set the id the load needs.
///
/// # It is `karakuri-cli`'s own route and not a second one
///
/// `taken_in_file` resolves a `.kset` with `setfile::bundle_authored` — which
/// is `setfile::resolve` behind its wall, and then the inlining — and hands the
/// result to `setfile::unbundle`. **Resolved *and* inlined rather than resolved
/// alone**, for that function's stated reason: `unbundle` writes a metadata
/// card for each source the lines carry, and handing it resolved lines with
/// nothing inlined would file the Set and leave every artifact cardless. Two
/// routes into one store that reach two different stores is the disagreement a
/// second spelling always is.
///
/// **Both spellings, and the branch is that function's too.** A `.kbset` has
/// its sources inside it and is read straight off the disk as lines; a `.kset`
/// names its parts by relative path and is resolved first. The `presets` scope
/// lists only the second form, so this branch was not reachable until a folder
/// row could be pressed — `console.html`: *"A Set file and a bundle are the
/// same file … so the scope draws one kind of row rather than two"*, and the
/// difference between them is a property of a file rather than a kind of row.
///
/// **The two binaries have no library target between them**, which is why this
/// is a second spelling of `karakuri-cli`'s eight lines rather than a call to
/// them, and it is written down here rather than left to be discovered: the
/// branch is the same branch and the two must not come apart the day a third
/// form arrives.
///
/// **The wall is `resolve`'s and not this file's**: a part named from outside
/// the file's own directory is refused, by path, because *"a Set somebody
/// handed you is not a way of asking this machine for its files"* (ADR-0229).
/// Nothing here loosens it and nothing here repeats it.
///
/// # An id this store already holds is refused, and the refusal is not written
/// # here
///
/// `setfile::unbundle` asks what the store holds before it writes a byte and
/// refuses an id that is taken — *"the id came from the file rather than from
/// you"* — and that sentence is the one the operator gets. A second check here
/// would be a second answer to *may this be overwritten*, and the two would
/// disagree the day one of them moved. What this adds is which of the two acts
/// failed: nothing was taken in, so nothing was loaded, and the deck is exactly
/// as it was.
///
/// # The id is the file's own
///
/// Read off the `set` record in the resolved lines rather than taken from the
/// row's word, because that is the id `unbundle` files it under and the id
/// `my sets` will list. They are the same word in `examples/`, and a preset
/// whose file says otherwise would otherwise be loaded by a name the store does
/// not hold.
pub(crate) fn taking_in(
    root: &std::path::Path,
    from: Taking<'_>,
    row: &str,
) -> Result<TakenIn, String> {
    // **Asked again rather than kept**, which is [`listing`]'s shape: the rows
    // crossed into the console as words, and the file behind a word is found
    // by asking the library again on the press. A second copy of the listing
    // held on this side is a copy that goes on naming a file that has moved.
    let file = from.file(row)?;
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    // **The form is the file's own and the branch is `karakuri-cli`'s** — see
    // this function's head. A name is what says which, and nothing is opened
    // to ask: the two suffixes are the two `folder_files` lists.
    let authored = file
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX));
    let lines = match authored {
        true => karakuri_environment::setfile::bundle_authored(&store, &file)?,
        false => karakuri_store::ndjson::read(&file)
            .map_err(|e| format!("reading `{}`: {e}", file.display()))?,
    };
    let id = lines
        .iter()
        .find_map(|line| match line.record() {
            Record::Set { id, .. } => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "`{}` carries no `set` record, so it names no id to file itself under",
                file.display()
            )
        })?;
    let said = karakuri_environment::setfile::unbundle(&store, &lines)?;
    Ok(TakenIn { file, id, said })
}

/// **The two rows of the vocabulary one press on a `presets` or a `folder` row
/// performs**, in the order they happen.
///
/// # Two operations because they are two rows of the page, and one press
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, so
/// loading a Set out of presets or out of a folder **is** that row performed.
/// The load after it is *Load material into a deck*, which is a different row
/// with a different operation. **One press, two rows** — `console.html` says
/// why it is one press: *"That is one press rather than two because taking it
/// in is what gives it the name the load needs."*
///
/// So the press emits both. Emitting only the load would be a press that
/// performs two of the page's rows and names one, and the row it dropped would
/// be the one **nothing in this workspace constructs**.
///
/// # Naming what a surface performed is the scope's rule, not a new one
///
/// `space` on the Library's head steps the mark itself and emits
/// `Operation::SelectScope` anyway, *"so that the press is recorded as `Silent(Surface)` rather than as
/// nothing at all"*. This is that, one key along: `written` answers
/// `Silent(NoRecord)` for a transfer, nothing in [`App::performed`] performs
/// one, and the emission is the naming.
///
/// # And it is not the key badge
///
/// `key_column::ROWS` maps `enter` in the Library to *Load material into a
/// deck* alone, and that
/// stays true: what an operator reaches from the keyboard is a load, and the
/// taking-in is what a load off `presets` does on the way. ADR-0213's
/// distinction is between an operator **reaching** an operation and something
/// **happening**, and constructing an operation is neither — which is
/// `panel_column.rs`'s own sentence, *"construction is not reachability, and
/// reachability is the definition."*
///
/// The transfer names the **file**, because that is what
/// `SetTransfer::Take` carries — *"a path because a file is what the only
/// existing route takes"* — and the load names the **id**, which is the file's
/// own `set` record rather than the row's word. They are the two halves of
/// [`TakenIn`] and neither is derived from the other here.
pub(crate) fn taken_in_press(deck: u8, taken: TakenIn) -> [Operation; 2] {
    [
        Operation::TransferSet {
            transfer: SetTransfer::Take { file: taken.file },
        },
        Operation::LoadSet {
            deck,
            set: taken.id,
        },
    ]
}

/// **The other direction of that row: a Set out of this store and into a file
/// the operator names**, asked for and answered without a frame waiting on
/// either half.
///
/// # The dialog is asked for here and awaited nowhere
///
/// `rfd::AsyncFileDialog::save_file` is called on this thread — the main one,
/// which is where a press handler is — and returns a future at once. On macOS
/// what that call has already done is `beginSheetModalForWindow:`, an
/// **asynchronous** sheet hung on this window: the run loop is untouched, so
/// the frame loop goes on drawing behind it and the panel's continuous motion
/// goes on saying *this is live*. That is the whole of what
/// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
/// asks of a mechanism that could run during a performance, and it was
/// **measured** rather than assumed before this was written —
/// [ADR-0311](../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)
/// carries the reading and the probe. The synchronous `FileDialog::save_file`
/// is the thing this must not be: it is `runModal`, a nested run loop, and a
/// panel that stops drawing.
///
/// **The future is awaited on a worker and so is everything after it**, which
/// is [`Keeping::save_set`]'s thread one act along and for its reason: a
/// bundle is a store read and every source inlined, then a file written, and
/// none of that is a thing to do on a frame (P-0091). The thread is detached
/// and no frame waits for it; the outcome comes back down a channel and is
/// said where a keep's is.
///
/// **The store is opened on the worker rather than handed in**, exactly as
/// [`Save::run`] does it: a `Store` is not what crosses the thread, a root is.
///
/// # Where the dialog opens, and what it is called
///
/// The name offered is `<id>.kbset` — the store's own naming rule, so nothing
/// is invented ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md))
/// — and the directory is the one the Library bay is pointed at where a folder
/// has been dropped on this window (ADR-0275), and the platform's own default
/// where none has. **A file already there is the dialog's question and never
/// this program's**: asking again on this side would be two programs asking
/// one question, and the operator would have answered the wrong one first.
pub(crate) fn sending(
    window: &Arc<Window>,
    root: &std::path::Path,
    folder: Option<&std::path::Path>,
    id: &str,
    tx: std::sync::mpsc::Sender<Sent>,
) {
    let mut dialog = rfd::AsyncFileDialog::new().set_file_name(format!(
        "{id}{}",
        karakuri_store::store::Store::SET_FILE_SUFFIX
    ));
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    let asked = dialog.set_parent(&**window).save_file();
    let (id, root) = (id.to_owned(), root.to_path_buf());
    std::thread::spawn(move || {
        let answer = pollster::block_on(asked).map(|handle| handle.path().to_path_buf());
        let _ = tx.send(sent(&root, id, answer));
    });
}

/// **What the dialog's answer comes to**: a file written, or nothing at all.
///
/// Split out of [`sending`]'s thread so that the half with no window in it can
/// be run without one — the dialog is the platform's and the answer is a
/// `PathBuf` or it is `None`, which is the whole of what this needs to know.
///
/// **`None` writes nothing and nothing is opened**: the store is not read, no
/// bundle is built and no path is touched. That is the property `a_dismissed_dialog_writes_nothing_and_says_so`
/// is watched to fail against, and it is why the early return is here rather
/// than inside a `map` over the write.
pub(crate) fn sent(root: &std::path::Path, id: String, to: Option<std::path::PathBuf>) -> Sent {
    let Some(to) = to else {
        return Sent {
            id,
            to: None,
            outcome: Ok(()),
        };
    };
    let outcome = bundled(root, &id).and_then(|text| {
        std::fs::write(&to, text).map_err(|e| format!("writing `{}`: {e}", to.display()))
    });
    Sent {
        id,
        to: Some(to),
        outcome,
    }
}

/// **One Set as the bytes of a `.kbset`**, which is `karakuri-cli`'s
/// `packaged_set` with the authoring half taken out.
///
/// This side never packages a `.kset`: the id half is the whole of what a row
/// of a library listing can name, and the flag's other spelling is *take in
/// then send in one flag* ([ADR-0260](../../../docs/adr/0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)),
/// which is two presses here and already reached.
///
/// **`setfile::bundle` is where the inlining and its one refusal live**
/// (ADR-0231): one missing artifact refuses the whole thing and names the
/// node, because a bundle short of a procedure looks self-contained and is
/// not. Nothing here repeats that and nothing here loosens it.
pub(crate) fn bundled(root: &std::path::Path, id: &str) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    Ok(setfile::bundle(&store, id)?
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// **Put a library Set on a running deck**, which is the whole of what
/// `Operation::LoadSet` needed and is a re-point rather than an install.
///
/// # It writes files and sends a description, and it builds nothing
///
/// `Deck::install` is the one function that puts a built Set in a slot, and it
/// is *"deliberately not reachable from a key or a surface: a live run changes
/// its material by editing a file and letting the worker build it, which is
/// what the budget watchdog is attached to."* So this does what an operator
/// with an editor does, in one press: it reads the Set out of the store,
/// writes every procedure in it into the scratch, and tells that slot's
/// watcher to look there instead. **Everything after this line is the path a
/// save already takes** — the worker compiles off the render thread, the swap
/// lands at a frame boundary, and the watchdog judges it there on what one
/// frame of that Set costs and rolls it back on its own if that is over the
/// budget. The
/// library gets all of that for nothing, and no second route into a slot is
/// opened.
///
/// **Nothing here is on the frame path.** A store read, a `setfile::load` that
/// checks every procedure, and up to a handful of small writes — on the press,
/// which is where this file already reads a directory (`arrangement`), and
/// never on a frame (P-0091). The compile is the worker's.
///
/// # The scratch name carries the slot, and it is the directory's rule now
///
/// `scratch::place` writes `<store>/scratch/<name>.kir` and overwrites what is
/// there, so two decks loading two Sets whose procedures happen to share a
/// name would be one file: the second load would move the first deck as well,
/// on its watcher's next poll, and nothing would say why. The name is
/// therefore `A0-drift.kir` — the deck letter, the node's place in the Set,
/// and the procedure's own name — which is unique per slot **and** per node,
/// stays readable in an editor, and says which deck an open file belongs to.
///
/// **It is `scratch::node_name` rather than a `format!` here**, because that
/// argument was never about loading. It is about two decks and one directory,
/// which is every run: every slot is materialised under the same spelling at
/// startup ([`working_copies`]), so a load writes into a directory already
/// laid out this way and a second spelling would be a second answer.
///
/// # What the aim states, and why all of it
///
/// [`watch::Aim`] is `Watch::new`'s argument list less the slot, and every
/// field here is read off the Set file rather than left at this program's
/// startup value — which is the failure each of `Watch`'s own fields is
/// documented against, and which would not show on the load at all. A
/// layering, a fold or a camera left behind is a slot that loads correctly and
/// then comes back as a different picture on the first later save.
///
/// The authorities are the one exception and are empty: `Record::Authority` is
/// deliberately not Set-file state, so a Set carries no grants and a load
/// starts a slot with none — which is what `--load-set` gives one.
///
/// The `Err` is a sentence for the operator. Every way this fails leaves the
/// deck exactly as it was: a store that will not open, a Set that is not
/// there, a procedure in it that no longer checks, a scratch that will not be
/// written, or a worker that has gone.
pub(crate) fn loading(
    root: &std::path::Path,
    slot: usize,
    salt: u32,
    // **The slot's [`Aiming`] and not its sender**, so that where the watcher
    // is pointed is kept with what was sent. A load that sent an aim and left
    // `Aiming::at` behind would leave the next rewiring restating the pair the
    // run launched with — see [`Aiming`].
    aim: &mut Aiming,
    id: &str,
) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let loaded = karakuri_environment::setfile::load(&store, id)?;
    // Said rather than swallowed: a note is the reader telling the operator
    // what it did with a file it could only partly honour, and a load that
    // quietly ignored one is a picture nobody can account for.
    for note in &loaded.notes {
        println!("  load: {note}");
    }

    let letter = deck_letter(slot as u8);
    let mut named = Vec::with_capacity(loaded.srcs.len());
    for (at, ((checked, src), name)) in loaded.nodes().zip(loaded.node_names()).enumerate() {
        let path = karakuri_environment::scratch::place(
            root,
            // **The directory's one naming rule, asked rather than spelled
            // again.** It was written out here when this was the only thing in
            // this program that wrote into the scratch; every slot is
            // materialised at startup now, so a second spelling of
            // `A0-drift.kir` would be a second answer to what a scratch file
            // is called — and the two would disagree on the day one of them
            // moved.
            &karakuri_environment::scratch::node_name(slot, at, &checked.name),
            src,
        )?;
        // **The Set file's node name and not the procedure's**, which is
        // `--load-set`'s own pairing: an `edge` in the file resolves against
        // the name the file wrote, and a rebuild that called the node whatever
        // its procedure declares would break the slot on its first save.
        named.push(karakuri_environment::compile::Named { name, path });
    }
    let mut named = named.into_iter();
    let head = named
        .next()
        .ok_or_else(|| format!("`{id}` names no procedure at all"))?;

    // **The file's salts, and a derived one where it recorded none** — the
    // rule `karakuri-cli`'s `salts_for` follows, restated here because that
    // program has no library target. The seed is the file's first salt where
    // it has one and this slot's own where it has not, so a Set that recorded
    // its colours comes back with them and one that did not is salted like the
    // slot it landed in.
    let seed_salt = loaded.salts.first().copied().flatten().unwrap_or(salt);
    let salts: Vec<u32> = (0..loaded.l1s.len())
        .map(|at| {
            loaded
                .salts
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed_salt, at))
        })
        .collect();

    let nodes = loaded.srcs.len();
    aim.re_point(watch::Aim {
        head,
        rest: named.collect(),
        layering: loaded.layering,
        live: loaded.live,
        // **The first geometry's recorded capacity over all of them**, which
        // is what `--load-set` folds into `--capacity` and is `Watch`'s own
        // shape: one `Option<u32>` for the slot, because a rebuild recompiles
        // the files and each geometry's own declaration is the default. A Set
        // that recorded two different capacities loses the second, which is a
        // limit this program shares with the command line rather than one it
        // invented.
        capacity: loaded.capacities.first().copied().flatten(),
        seed_salt,
        salts,
        // A Set holds a built-in camera whatever its files declare, so
        // `Orbit::default()` where the file recorded none is the camera it
        // would have been built with rather than a value invented here.
        camera: loaded.camera.unwrap_or_default(),
        overrides: loaded.params,
        // Nothing in a Set file publishes a control — `setfile` writes none
        // and reads none — so this is empty for the same reason this program's
        // startup watchers pass an empty list: there is no `--publish` here to
        // fold in either (ADR-0216).
        published: Vec::new(),
        bindings: loaded.bindings,
        edges: loaded.edges,
        authorities: Vec::new(),
        // **What this slot is now running, and what every version it writes
        // from here on is filed under.** It is the id the operator picked out
        // of the library, carried on the aim because that is what a re-point
        // moves: left off, the loaded Set's whole chain would go on being filed
        // under the material the slot was running before the press, in names
        // nothing reads back (ADR-0276).
        set: Some(id.to_string()),
    })
    .map_err(|()| {
        format!(
            "deck {letter}'s build worker is gone, so `{id}` cannot be built; \
             what is on that deck keeps running"
        )
    })?;
    Ok(format!(
        "  load: deck {letter} <- `{id}` ({nodes} node{}) -> written into {}/{} and its watcher \
         re-pointed; the worker builds it and the budget judges it",
        match nodes {
            1 => "",
            _ => "s",
        },
        root.display(),
        karakuri_environment::scratch::DIR,
    ))
}

/// **Every arrangement the store holds, by name**, for the pill's menu to
/// list — [`library`] over the fourth directory rather than the first.
///
/// Its two rules are this one's, said again because they are the same two: a
/// store that is not there is listed as nothing and **is not created**, since
/// a program that listed a menu by first making a store would change the
/// directory it was run in; and a store that could not be read says so, since
/// a menu that is empty because the directory would not open looks exactly
/// like one that is empty because nobody has saved.
///
/// **Read when it changes rather than per frame.** Once at startup, and again
/// after a save lands — which is the only thing in this program that adds a
/// name. `Store::list_arrangements` sorts by name, so the menu draws what it
/// is handed and sorts nothing.
pub(crate) fn arrangements(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    match Store::open(root).and_then(|store| store.list_arrangements()) {
        Ok(filed) => filed.into_iter().map(|entry| entry.name).collect(),
        Err(e) => {
            println!("arrangements: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// **A star put on a Set or taken off it**, and the second route in this
/// program that both reads an operation and reaches a disk.
///
/// # It is [`arrangement`]'s shape and sits beside it for its reason
///
/// The panel cannot reach the store (ADR-0156) and the store cannot reach the
/// panel, so the two halves meet in a third party and this file is it. It
/// answers `None` for every other operation, which is what lets it sit on the
/// one path an emitted operation already takes rather than being a second
/// route into the store.
///
/// # What it writes, and what it deliberately does not
///
/// `Store::set_favourite` — one stat, one atomic write of
/// `<store>/favourites.json`, and the whole of the layout question is
/// ADR-0299's rather than this file's. **Nothing here re-lists**: the marks
/// the bay draws and the rows `my sets` holds are both [`listing`]'s answer,
/// and a second derivation here would be a second answer to *what is starred*
/// with a file write between them. The caller re-lists on the same branch it
/// re-lists a scope press on.
///
/// # Who asked decides where it lands, and for a star there is nowhere else
///
/// [P-0096](../../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// is the actor and not the flag, and `my sets` is by construction the list of
/// Sets **the operator chose** — so a model's star must not reach
/// `<store>/favourites.json`. That much is
/// [ADR-0261](../../../docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)'s
/// rule applied one control along, and it is why this branches on [`Asked`]
/// exactly as `karakuri_environment::filed_as` does for a save.
///
/// **Where the two part company is the second directory.** A model's save
/// lands in `<store>/sandbox/` because what lands there is *material* — an
/// edit-history snapshot an operator goes looking for after a show — so
/// refusing it would lose an evening of work. A star is one bit whose whole
/// meaning is *this row appears under `my sets`*, so a sandbox favourites file
/// would be a list no scope lists, no tool reads and the operator never sees,
/// while the model was told it had succeeded. **So a model's star is refused
/// out loud** (ADR-0301), and the refusal names the id and says where the Set
/// is — which is the same shape the class pills' refusals take, so that a
/// model can tell the person beside it which mark to press.
///
/// **`Standing::Open` stays and `gate.rs` is untouched.** The refusal is the
/// performer's and not the gate's, exactly as a model's save is not refused at
/// the gate but filed somewhere else by whoever performs it.
///
/// **The model arm is written before the route is**, which is [`arrangement`]'s
/// own position: no tool publishes `SetFavourite` today, the page's MCP badge
/// is `plan`, and a control that arrives at this function finds the rule
/// already here rather than adding it.
///
/// # The three things it can say, and each is said out loud
///
/// **The state was already the one asked for**, which is `Ok(false)` and is an
/// ordinary answer rather than a refusal: the operation names a state and not
/// a toggle, so a second press of *star this* says the same thing again and
/// the file's own time is not touched. **The Set is not one this store
/// holds**, which is `StoreError::NoSet` carrying the id back
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md))
/// — a `presets` or a `folder` row is a file rather than a Set of this
/// library's, and starring one is refused with the sentence saying so.
/// **Taking a star off is never refused**, which is the asymmetry that makes a
/// mark left behind by a file somebody deleted clearable from the row it no
/// longer draws.
///
/// **Never panics**, for [`arrangement`]'s reason: a panic reachable from an
/// event handler aborts this process rather than unwinding.
pub(crate) fn favourite(
    root: &std::path::Path,
    asked: Asked,
    operation: &Operation,
) -> Option<String> {
    let Operation::SetFavourite { id, favourite } = operation else {
        return None;
    };
    if asked == Asked::Model {
        return Some(format!(
            "star: `{id}` was not {} — `my sets` is the list of Sets the operator chose, and a \
             star is theirs to put on: it is one press on the mark at the left of that row in \
             the Library bay, under `all`. The Set itself is untouched and is listed there",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ));
    }
    let wrote = Store::open(root).and_then(|store| store.set_favourite(id, *favourite));
    Some(match wrote {
        Ok(true) => format!(
            "star: `{id}` {} — `my sets` is the starred subset of `all`, and this row is {} \
             it",
            match favourite {
                true => "is starred",
                false => "has its star off",
            },
            match favourite {
                true => "in",
                false => "out of",
            }
        ),
        Ok(false) => format!(
            "star: `{id}` was already {}, so nothing was written",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
        Err(e) => format!(
            "star: `{id}` was not {}: {e}",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
    })
}

/// **Where a named arrangement is kept and put back**, and the one route in
/// this program that both reads an operation and reaches a disk.
///
/// # Why it is here, in a package neither side depends on
///
/// The panel cannot reach the store. `karakuri-console` dropped
/// `karakuri-store` when this program moved out of it, and the drop was the
/// point — a crate that takes no device and no disk is what ADR-0156 bought,
/// and its manifest now has no entry that could be reached for at all. The
/// store cannot reach the panel either: `karakuri-store`'s `src/` must not
/// name `karakuri-layout`, so it keeps an arrangement as bytes it does not
/// understand, exactly as it keeps `.kir` source
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §4). **So the two halves meet in a third party, and this file is the third
/// party** — the same position it holds for a record, where the vocabulary
/// says what to write and only somebody holding a `Deck` can apply it
/// ([`apply`]).
///
/// # The route, and where each half of it is decided
///
/// - **Saving** is `serde_json::to_vec` of [`Panel::layout`] into
///   `Store::write_arrangement`, which is ADR-0221's own sentence. The format
///   is `karakuri-layout`'s hand-written `Serialize`, so an unbounded maximum
///   goes out as an explicit absence rather than as an infinity JSON cannot
///   spell, and a `NodeId` goes out as the bare number it is.
/// - **Putting one back** is `Store::read_arrangement`, `serde_json` into a
///   [`Layout`], and [`Panel::restore`]. **Neither this file nor the panel
///   checks the arrangement**: `Layout`'s `TryFrom<Wire>` is the one place a
///   file that disagrees with itself is refused rather than repaired
///   (ADR-0158), and a check here would be a second answer to a question that
///   already has one.
///
/// # What it does with each of the three ways it can fail
///
/// **Says it and moves nothing**, and never panics: a panic reachable from an
/// event handler aborts this process rather than unwinding (see the module
/// documentation). The three are a store it could not open or write, a name
/// nothing is filed under, and a file that will not read back — and the third
/// is the one that has to be told apart from the second, because *there is no
/// such arrangement* and *the arrangement you saved is broken* send an
/// operator to two different places.
///
/// **A name nothing is filed under never falls back to the default.**
/// `Store::read_arrangement` answers `StoreError::NoArrangement(name)` and
/// that sentence carries the name, which is the whole reason the store has a
/// fourth error variant rather than reusing `NotFound`: an operator who
/// mistyped a name needs to be told the name, not to watch their console reset
/// (ADR-0221 §2).
///
/// # The store is created by a save and not by a restore
///
/// [`library`] refuses to create one, because *"a program that listed a
/// library by first making one would change the directory it was run in"*, and
/// a restore is a read on exactly those terms. A **save** is the case
/// `Store::open` establishing the layout is right for — it is a program that
/// is about to write — so the two halves below differ, deliberately, and the
/// restore's guard is what keeps `cargo run -p karakuri` in somebody's home
/// directory from leaving a `.karakuri` behind for having asked a question.
///
/// # It answers `None` for every other operation
///
/// Which is what lets it sit on the one path every emitted operation already
/// takes ([`App::performed`]) rather than being a second route into the
/// panel. **The transport row's arrangement pill emits both**, and the
/// manual's two rows say it is the only one of the four surfaces that can: a
/// `panel` badge each and three empty ones, because a name is what a key
/// press, a map line and an unpublished tool each have no way to say. This
/// wiring was written before that control existed — exactly as [`unwritten`]
/// is written for controls that do not exist yet — and the control is what
/// arrived at it.
pub(crate) fn arrangement(
    root: &std::path::Path,
    panel: &mut Panel,
    arr: &mut view::Arrangement,
    operation: &Operation,
) -> Option<String> {
    match operation {
        Operation::SaveArrangement { name } => {
            let (line, kept) = keep_arrangement(root, panel, name);
            // **The name in use moves only when the file did.** A save that
            // was refused leaves the pill saying what it said, because
            // nothing under that name is on the disk — and the listing is
            // re-read only then, since a refusal added no name to it.
            if kept {
                arr.name = Some(name.clone());
                arr.filed = arrangements(root);
            }
            Some(line)
        }
        Operation::RestoreArrangement { name } => {
            let (line, back) = put_arrangement_back(root, panel, name);
            if back {
                arr.name = Some(name.clone());
            }
            Some(line)
        }
        _ => None,
    }
}

/// The running arrangement, filed under `name` — and whether it landed. See
/// [`arrangement`].
pub(crate) fn keep_arrangement(
    root: &std::path::Path,
    panel: &Panel,
    name: &str,
) -> (String, bool) {
    if let Err(refusal) = checked_name(name) {
        return (refusal, false);
    }
    let bytes = match serde_json::to_vec(panel.layout()) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                format!("arrangement: `{name}` was not kept — it did not serialise: {e}"),
                false,
            )
        }
    };
    let wrote = Store::open(root).and_then(|store| store.write_arrangement(name, &bytes));
    match wrote {
        Ok(()) => (
            format!(
                "arrangement: kept as `{name}` — {} bytes at {}",
                bytes.len(),
                root.join("arrangements")
                    .join(format!("{name}.arrangement.json"))
                    .display()
            ),
            true,
        ),
        Err(e) => (format!("arrangement: `{name}` was not kept: {e}"), false),
    }
}

/// **The one place a typed arrangement name is refused**, and the reason it is
/// here rather than in the pill that took the letters.
///
/// `<name>` becomes one path component under `<store>/arrangements/`, and
/// `karakuri-store` says outright that **nothing there checks it**: *"`<name>`
/// becomes one path component and that is the caller's rule to keep"*
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §1, which names letters, digits, `-` and `_`). So `../../elsewhere` is a
/// path, and a path never reaches that call from here.
///
/// **The surface owns the affordance and never the authority**
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)):
/// the pill takes whatever is typed and this is where it meets the wall, so a
/// name refused by a hand and a name refused by anything else that ever
/// reaches this operation meet the same one. A pill that silently dropped the
/// characters it did not like would be a rule an operator could only find by
/// experiment — which is the failure the console page names about a control
/// that quietly declines.
///
/// **It says the same three things `mcp::checked_id` says about a Set id**,
/// which is [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// as far as it can be kept today and no further: that function is private to
/// `karakuri-environment`'s `mcp` module and its sentences say `id` and
/// `<store>/sets/`, so it cannot be called from here and could not be quoted
/// if it were. **When an arrangement name gets a second surface — a map line,
/// an MCP tool, a `--restore-arrangement` flag — the two collapse into one
/// shared `checked_name`, and this comment is where whoever does it should
/// start.**
pub(crate) fn checked_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(
            "arrangement: nothing was typed, and an arrangement is filed under a name — \
             the default arrangement is the one that has none"
                .to_owned(),
        );
    }
    match name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        Some(bad) => Err(format!(
            "arrangement: `{name}` holds `{bad}`, and an arrangement name is letters, digits, \
             `-` and `_`: it is one path component and it names a file under \
             `<store>/arrangements/`"
        )),
        None => Ok(()),
    }
}

/// The arrangement filed under `name`, into the window the panel already has.
/// See [`arrangement`].
pub(crate) fn put_arrangement_back(
    root: &std::path::Path,
    panel: &mut Panel,
    name: &str,
) -> (String, bool) {
    if !root.is_dir() {
        return (
            format!(
                "arrangement: no store at {}, so nothing is filed under `{name}` — and the \
                 console has not moved",
                root.display()
            ),
            false,
        );
    }
    let bytes = match Store::open(root).and_then(|store| store.read_arrangement(name)) {
        Ok(bytes) => bytes,
        Err(e) => return (format!("arrangement: `{name}` is not back — {e}"), false),
    };
    let layout: Layout = match serde_json::from_slice(&bytes) {
        Ok(layout) => layout,
        // **Refused whole rather than repaired**, which is the loader's own
        // sentence and not this file's judgement (ADR-0158).
        Err(e) => {
            return (
                format!(
                    "arrangement: `{name}` is not back — the file disagrees with itself and is \
                     refused rather than repaired: {e}"
                ),
                false,
            )
        }
    };
    let viewport = panel.layout().viewport();
    panel.restore(layout);
    (
        format!(
            "arrangement: `{name}` is back, at the {} x {} this window already had",
            viewport.w, viewport.h
        ),
        true,
    )
}

/// **What the mixer strips read this frame**: one per slot the deck has, out
/// of the six things a `Deck` will say about a slot.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one:
/// `slot_count`, `residency`, `gain`, `opacity`, `blend`, `mask` and `level`
/// are its own, and this takes a `&Deck` because it is on the side of the seam
/// that is allowed one — what crosses into the console is a name, a word and
/// four numbers (ADR-0156).
///
/// # Where each one comes from, and the two that are not the deck's
///
/// - **The tally** is `Deck::residency`, which is the **effective** residency
///   the frame loop reads and not `requested_residency`. The governor moves a
///   slot down without anybody asking, and a tally showing the request would
///   be describing a slot that is doing something else.
/// - **And the request beside it** — `Deck::requested_residency`, which is the
///   other half of the same pair. Both are handed over and **neither side
///   computes `Deck::is_parked`**: the engine has the predicate and the
///   console derives its own from the two values (`view::Strip::pending`), so
///   what crosses the seam stays a residency and a residency rather than
///   becoming a bit whose meaning is written down in only one of the two
///   crates. It is the same reading as the four numbers below — the deck says
///   what it is doing, and the surface decides what that looks like.
/// - **The trim and the fader** are `gain` and `opacity`, which are two
///   controls and not one — *"opacity at zero silences under every blend mode,
///   gain at zero does not silence `over`"* — and the bay draws them as two.
/// - **The blend** is [`blend_mode`]: the engine's `Blend` turned into the
///   vocabulary's `BlendMode`, because the chip is a control now and a control
///   has to know which of the three it is on to say what the next one is
///   (ADR-0187). **This is where a fourth engine mode with no operation
///   variant stops the build**, which is the failure worth having — the
///   alternative is a word drawn on a chip no map can ask for.
/// - **The mask** is `Deck::mask(slot).kind()` for the mark, **and its angle
///   beside it** — `view::Strip::mask_angle`, which is read to build the
///   press's operation and drawn nowhere
///   ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///   The position and the softness are left behind: the strip's `.mini` says
///   *which shape*, three numbers about that shape are an inspector row, and
///   the two the record needs are read where the record is written rather
///   than carried across this seam.
/// - **The level** is `Deck::level`, which is already `None` for every case
///   where a held reading would be about a different image. Its
///   `frames_behind` is not passed on — `view::Level` is where that argument
///   is written out, and the short of it is that the number means different
///   things on different loops and this loop is a `Fifo` one that never stops
///   asking for frames while anything is live.
/// - **The name** is [`material`], and it is this file's because a `Set` has
///   none. See `view::Strip::name`.
///
/// # Written into the `Vec` the view already holds
///
/// `out` is grown to the deck's slot count and then every field of every strip
/// is written, so nothing a `push` left behind is ever read. The name is the
/// one field that owns anything, and it is rewritten only when it differs —
/// which keeps this off the frame's allocation budget (ADR-0164) rather than
/// putting a `String` per strip on it every frame.
/// **What a scheduled move on this control is taking it to**, or `None` for a
/// control nothing is moving.
///
/// The engine's `Transition` carries the instant it starts, its length in
/// beats and its curve as well, and none of the three crosses this seam: the
/// console has no beat count, so what it could draw out of them is nothing.
/// See `view::Strip::gain_to`.
pub(crate) fn destination(deck: &Deck, slot: EngineSlot, control: Control) -> Option<f32> {
    deck.transitions_on(slot)
        .find(|t| t.control() == control)
        .map(|t| t.to())
}

/// **What each preview cell's risk badge reads**, from the pass that decided
/// it — one entry per deck slot, in slot order.
///
/// `Decision::budgeted_ms` is the number the governor spent and
/// `Decision::basis` says which of its two numbers that is (ADR-0296). The
/// console reads the number into five bands and carries the basis undrawn
/// (ADR-0298), so both halves cross and neither is spent twice.
///
/// **`Basis::Unbudgetable` is written as `None`**, and that is the whole of
/// what this function decides. It is a slot nothing measured and nothing
/// estimated, it is not a zero, and a zero here would draw a **green** dot.
///
/// **A deck with more slots than the row has cells contributes nothing past
/// the fourth**, which is the reading `View::select` refuses a key on.
pub(crate) fn costs(governed: &Report) -> [Option<Budgeted>; DECKS] {
    let mut out = [None; DECKS];
    for decision in &governed.decisions {
        let Some(cell) = out.get_mut(decision.slot) else {
            continue;
        };
        *cell = match (decision.budgeted_ms, decision.basis) {
            (Some(ms), Spent::Estimated) => Some(Budgeted {
                ms,
                basis: Basis::Estimated,
            }),
            (Some(ms), Spent::Measured) => Some(Budgeted {
                ms,
                basis: Basis::Measured,
            }),
            // `Unbudgetable`, and a number arriving without a basis — which
            // cannot happen, and is not worth inventing a band for if it does.
            _ => None,
        };
    }
    out
}

pub(crate) fn mixer(deck: &Deck, names: &[String], out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: String::new(),
            tally: view::Tally::Allocated,
            requested: view::Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        // **One name per slot, because a load moves one slot.** It was one
        // name for the whole deck while every slot ran the same pair and
        // nothing could change any of them; a library Set loaded into deck
        // B would then have left every strip reading the pair this program was
        // launched with, which is a readout that is wrong and says nothing
        // (P-0094). Empty for a slot nobody named, which draws no name at all
        // rather than somebody else's.
        let name = names.get(slot).map_or("", String::as_str);
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        let slot = EngineSlot(slot as u8);
        strip.tally = tally(deck.residency(slot));
        strip.requested = tally(deck.requested_residency(slot));
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        // **Where a scheduled move is taking each fader**, which is
        // `Deck::transitions_on` — *"what is moving on this slot, for a status
        // line"* — read for a surface instead. `karakuri-cli`'s line prints
        // `o>0.80` off the same call and for the same reason: with the default
        // quantum a fade is armed up to a bar before it is due, and a control
        // that changes something invisible is indistinguishable from one that
        // is broken.
        //
        // **At most one per control**, because `Deck::schedule` cancels
        // whatever was moving that pair before it pushes — so `find` is the
        // whole answer rather than the first of several.
        strip.gain_to = destination(deck, slot, Control::Gain);
        strip.opacity_to = destination(deck, slot, Control::Opacity);
        strip.blend = blend_mode(deck.blend(slot));
        strip.mask = masked(deck.mask(slot).kind());
        // **Read for the press and painted nowhere**, which is what
        // `view::Strip::mask_angle` is for: the chip asks for a shape and the
        // operation carries an angle, so the angle it carries is the one the
        // slot already has (ADR-0203). A harness that left this at zero would
        // make every press straighten a diagonal front, and nothing on the
        // panel would show it having happened.
        strip.mask_angle = deck.mask(slot).angle();
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// **The Staging lane's rows, off the deck's own verdicts and the per-node
/// hashes its builds reported** — one per node a build changed, or one on the
/// slot where a verdict has no changed node behind it (ADR-0326).
///
/// # It drains, because a `HotSwap`'s events are a caller's to take
///
/// `HotSwap::events` is documented as the caller's — *"a caller that stops
/// draining eventually makes this grow"* — and until this program had a
/// producer there was nothing to drain, so nothing did. `pending_events` is
/// the other reading and is deliberately **not** what this uses: it is the
/// read-only view `Deck::begin_frame` takes *inside* a frame, and a surface
/// that read it without draining would be the caller bug the engine names.
///
/// **So the lane's state is kept here rather than re-derived per frame**, and
/// that is what a drain forces and is also what is wanted: the events are a
/// stream of verdicts and a slot's rows are the newest of them.
/// `view::View::staging` is written on the frames a build landed on and left
/// alone on every other, which is the same budget `mixer`'s name is kept to
/// (ADR-0164) with the frames-that-touch-it much rarer.
///
/// # What each verdict does to a row
///
/// - **`Swapped`, `Rejected`, `Overloaded`, `SourceRefused`** — the slot has
///   rows, on `view::Stage`'s four words. Whether the build is on screen and
///   running is what separates the first three — `Overloaded` is on screen and
///   stopped — and the fourth is the one where there was no build: the checker
///   turned the source down, so the row carries what it said as well as the
///   word (ADR-0310). **How many rows is the diff**: `Swapped` and
///   `Overloaded` draw one per node the build changed, and `Rejected` and
///   `SourceRefused` draw one on the slot, because a build that did not happen
///   has no node list to hold against the one before it.
/// - **`Accepted`** — the watchdog says the version held the budget, so the
///   file and the picture agree and that slot's rows leave the lane. It is not
///   the operator's verdict, which is *keep* and is taste rather than cost:
///   the cost verdict is what clears a row an operator has not pressed, and
///   `view::staging` is where that substitution is argued.
/// - **`WorkerLost`** — the build worker panicked and nothing will be built
///   again this run. **No row changes**, and that is the reading rather than
///   an omission: every verdict already taken still stands, and there is no
///   verdict outstanding for the worker to have taken with it — one is reached
///   in the call its swap lands in. What is lost is the *next* build, and the
///   lane has never been where that is said — the engine prints it.
///
/// **A row is not removed when its slot is parked or its material is
/// replaced.** What takes a row off the lane is a verdict in the candidate's
/// favour, and residency does not reach one: a candidate is judged on its own
/// measured cost wherever the slot is (ADR-0313).
///
/// # It writes the transport's health capsule too, and the two are not the
/// same reading
///
/// `view::Transport::health` is *what the last write did* and the lane is
/// *what is still outstanding*, so the two disagree in both directions and
/// both are the mock's: a run in which the last build landed and was then
/// accepted draws `landed` in the transport with an empty lane, and a run in
/// which one slot was stopped while a later one landed draws `landed` in the
/// transport with an `overloaded` row under it. The capsule carries no deck
/// letter, which is why it can only say the second of those and why the lane
/// is where the address is.
///
/// **They are written from one drain because there is only one.**
/// `Deck::events` empties the channel; a second pass for the capsule would
/// read nothing at all, which is the same sentence the reporter above is
/// written under.
///
/// # It answers whether the live Set changed, because something else has to
/// know
///
/// `true` when a build was installed, which is the moment `Set::published` says
/// a console should re-read a slot — *"A console reads this when a Set lands,
/// not per frame."* [`inspector`] is what acts on it. **Every other event
/// answers `false`, the budget's verdict included**: since ADR-0316 neither
/// verdict replaces what is playing — one lets the installed Set run and the
/// other stops it where it is — and the install that did replace it was
/// reported by `Swapped` in the same drain.
///
/// # What it costs
///
/// **Nothing on a frame nothing was built on.** The event drain collects into
/// a `Vec` that does not allocate when it is empty, and every allocation below
/// that is inside the `for` over it.
///
/// **On the frame a build lands**, which is the frame that also installed a
/// whole Set built on the worker: a `Vec` of the changed addresses, a `Changed`
/// per one of them with its address and its name, and a `view::Candidate` per
/// row. A slot's rows are replaced whole rather than rewritten in place — see
/// [`settle`], where that is argued — so this is a handful of short strings
/// against a Set build, and a run in which nobody saves pays for none of it.
pub(crate) fn staging(
    deck: &mut Deck,
    // **What each slot is playing, and the channel that says what a build was
    // made of.** Here rather than taken up by the caller afterwards, because
    // the rows *are* the diff: a build's per-node hashes against the ones the
    // slot was already on is what says which nodes changed, and both lists are
    // in one hand exactly once — at the moment the build is taken up
    // (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    // A second pass would be a second answer to *what changed*, and the first
    // of the two lists is gone by then.
    keeping: &mut Keeping,
    // **Where each slot's watcher is pointed**, for the names a build's nodes
    // are spelled with — see [`built_nodes`]. A field of `Engine` beside the
    // deck rather than the whole of one, so the deck above can be borrowed
    // mutably at the same call.
    aims: &[Aiming],
    out: &mut Vec<view::Candidate>,
    // **What the transport row's health capsule reads**, off the same drain
    // and for the same reason the reporter is fed from here:
    // `Deck::events` empties the channel, so a second reader that came back
    // for it would find nothing.
    //
    // **Kept across frames rather than rewritten per frame**, which is what
    // makes it a different reading from the lane beside it: a row leaves the
    // lane when nothing is outstanding, and the last verdict there was does
    // not stop having happened. `Verdict::Settled` and `Verdict::Nothing`
    // therefore leave this alone — the watchdog saying a version held the
    // budget is not a write, and a lost build worker is not one either.
    health: &mut Option<view::Stage>,
) -> bool {
    let mut landed = false;
    for slot in 0..deck.slot_count() {
        let addr = EngineSlot(slot as u8);
        // **Taken out of the channel before anything else is asked of the
        // deck.** `Deck::events` borrows the deck for as long as it is being
        // read, and what the rows need afterwards is the Set the swap in this
        // same drain installed — `Set::node_names`, which is what a node is
        // called. An empty drain collects into a `Vec` that allocates nothing,
        // so a frame on which nothing was built pays for this in a branch.
        let events: Vec<Event> = deck.events(addr).collect();
        // **What the build that landed changed, kept across the drain**, so
        // that the `Overloaded` following its `Swapped` draws the same rows:
        // both are about one build and carry its id, and the diff is taken
        // once. Empty on every frame nothing was built.
        let mut diffed: Vec<(u64, Vec<Changed>)> = Vec::new();
        for event in events {
            // **Where the same event goes when somebody who is not at the
            // panel is watching**, and nothing for a run without `--mcp`. A
            // model that wrote a procedure has no other way to learn that its
            // slot was stopped for cost, and *it compiled* is not the same
            // news as *it is on screen and running*.
            //
            // **A checker's refusal goes out with every diagnostic it had**,
            // which is the round trip `docs/principles/0083-…` is about: the
            // reader at this end is a program writing the next attempt, and
            // the row beside it has room for the first line only.
            //
            // **Reported from here rather than from a second drain.**
            // `Deck::events` empties the channel, so a loop that read it again
            // would read nothing at all: the lane and the server are told by
            // one pass or one of them is told by none.
            if let Some(mcp) = keeping.mcp.as_ref() {
                mcp.swap(slot, &event.to_string());
            }
            // **What a swap changed, taken up here.** A `Swapped` is the only
            // event that changes what a slot is running (ADR-0316), so it is
            // the only one that has a diff to take — and the rows it draws are
            // that diff, one per node, named off the Set the swap installed.
            if let Event::Swapped { id, .. } = &event {
                let changed = keeping.took_up(aims, slot, *id);
                diffed.push((*id, changed_rows(deck.slot(addr).set(), &changed)));
            }
            // **Whether the *live Set* changed**, which is a different
            // question from whether a row did and is why this is read here
            // rather than off the rows: a build that landed replaced what is
            // playing, and nothing else does. A refusal changed nothing, and
            // neither verdict changes it either — one lets the installed Set
            // run and the other stops it where it is (ADR-0316).
            landed |= matches!(verdict(&event), Verdict::Waiting(_, view::Stage::Landed));
            // **The diagnostics, where there are any**, taken off the event
            // beside the verdict rather than carried through `Verdict`: that
            // type is `Copy` and is the mapping a CPU test asserts, and a
            // slice on it would make it neither.
            let said: &[String] = match &event {
                Event::SourceRefused { said, .. } => said,
                _ => &[],
            };
            // **Which nodes this verdict is about**, and empty for every
            // verdict that has none — a build that did not happen has no node
            // list to hold against the one before it, and a rebuild whose
            // stack came back identical has an empty diff. `settle` draws one
            // row on the slot for both, which is the same row and the same
            // reason (ADR-0326).
            let changed: &[Changed] = match &event {
                Event::Swapped { id, .. } | Event::Overloaded { id, .. } => diffed
                    .iter()
                    .find(|(built, _)| built == id)
                    .map_or(&[][..], |(_, rows)| rows.as_slice()),
                _ => &[],
            };
            match verdict(&event) {
                Verdict::Waiting(label, stage) => {
                    // **The newest verdict of the drain wins, whichever slot
                    // it came from.** The capsule is one word with no address
                    // on it — `console.html` draws it beside the frame
                    // readout and gives it no deck letter — so what it can
                    // honestly say is *what the last write did*, and one save
                    // of the pair this program watches builds every slot.
                    // Which slot each verdict was about is the lane's, and
                    // which node of it, and that is why both are drawn.
                    *health = Some(stage);
                    settle(out, slot, label, stage, said, changed)
                }
                // Nothing is outstanding on this slot any more, so it has no
                // rows. `retain` rather than an index: a slot has as many rows
                // as its last build changed nodes, and they go together.
                Verdict::Settled => out.retain(|row| row.deck != slot),
                Verdict::Nothing => {}
            }
        }
    }
    landed
}

/// **One node a build changed, as a lane row needs it** — the address a press
/// is spelled with, the address the row draws, and what that node is called.
///
/// **Three fields and not one, because the row and the operation want
/// different halves of the same fact.** `view::Candidate::at` is the payload
/// and `view::Candidate::addr` is the string, which is `view::Param`'s
/// arrangement one bay over; both are written here, in one place, so they are
/// filled together or not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Changed {
    pub(crate) at: karakuri_operation::NodeAddress,
    pub(crate) addr: String,
    pub(crate) name: String,
}

/// **The changed nodes of a build, turned into rows** — the addresses the
/// watcher reported, resolved against the Set that swap installed.
///
/// **The name is `Set::node_names`' and not the build's label**, which is the
/// whole of why this needs the Set at all: a label is every node's `proc` name
/// joined with ` + `, and a row that is one node wants that node's own. It is
/// the same name the Inspector writes on a node head — `Set::node_named` turns
/// each one back into the `(layer, index)` this compares against — so the two
/// bays call one node one thing.
///
/// **A node the Set does not name is passed over rather than drawn nameless.**
/// The two lists are the same build's, so this cannot happen from a rebuild;
/// what it would mean is that the Set and the report disagree about what the
/// slot holds, and a row invented out of that disagreement would be a press
/// aimed at an address nothing can resolve.
pub(crate) fn changed_rows(set: &Set, changed: &[(&'static str, u32)]) -> Vec<Changed> {
    if changed.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for name in set.node_names() {
        let Some((layer, index)) = set.node_named(name) else {
            continue;
        };
        if !changed
            .iter()
            .any(|(at, of)| *at == setfile::kind_name(layer) && *of == index)
        {
            continue;
        }
        rows.push(Changed {
            at: karakuri_operation::NodeAddress {
                layer: asked_layer(layer),
                index,
            },
            addr: node_addr(layer, index),
            name: name.clone(),
        });
    }
    rows
}

/// **A node's address as the mock's `.addr` spells it** — `L1:0`, `L2:0`,
/// `L4:0`.
///
/// [`layer_word`] is the layer half and this is the whole of it, written once
/// because two bays draw it: the Inspector's node heads and, since 2026-09-09,
/// the Staging lane's rows.
pub(crate) fn node_addr(layer: Layer, index: u32) -> String {
    format!("{}:{index}", layer_word(layer))
}

/// **Which of the four cells is showing a still**, in slot order —
/// `view::View::overloaded`, which the caption reads.
///
/// A slot the watchdog stopped holds the frame it last drew and is not stepped
/// or drawn (ADR-0316), and a held frame of good material is indistinguishable
/// from material: the word under the cell is what says which it is (ADR-0269).
///
/// **`false` past `slot_count`, not a panic.** The row is `view::DECKS` cells
/// whatever the deck holds, so the fourth cell of a three-slot deck asks about
/// a slot that is not there — and *there is no slot* is the caption's own word
/// rather than a state of one (`view::PREVIEW_NO_SLOT`).
///
/// A function rather than four lines in the frame loop, so that the bound is
/// written once and can be named from a test.
pub(crate) fn stopped_slots(deck: &Deck) -> [bool; view::DECKS] {
    std::array::from_fn(|slot| slot < deck.slot_count() && deck.overloaded(EngineSlot(slot as u8)))
}

/// **What one verdict does to the lane** — the whole of the mapping, in a
/// function a test can reach without a device.
///
/// A `match` and not a lookup, for [`blend_mode`]'s reason: a sixth
/// `swap::Event` stops the build here rather than being passed over by a
/// wildcard, and *what it does to the lane* is a question the person adding it
/// should have to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict<'a> {
    /// The slot has a row, under this name and on this word.
    Waiting(&'a str, view::Stage),
    /// The slot has no row: its file and its picture agree.
    Settled,
    /// The lane does not change.
    Nothing,
}

/// See [`Verdict`] and [`staging`], which is where each arm is argued.
pub(crate) fn verdict(event: &Event) -> Verdict<'_> {
    match event {
        Event::Swapped { label, .. } => Verdict::Waiting(label, view::Stage::Landed),
        Event::Rejected { label, .. } => Verdict::Waiting(label, view::Stage::Refused),
        Event::Overloaded { label, .. } => Verdict::Waiting(label, view::Stage::Overloaded),
        // **A row, because the file and the picture disagree** — which is the
        // whole of what this lane is for. Nothing was built, so nothing
        // replaced what is playing and the operator's newest edit is on disk
        // and nowhere else. The word is the checker's and not the build's:
        // `Refused` above is a Set that would not assemble.
        Event::SourceRefused { label, .. } => Verdict::Waiting(label, view::Stage::NotCompiled),
        Event::Accepted { .. } => Verdict::Settled,
        Event::WorkerLost => Verdict::Nothing,
    }
}

/// **One slot's rows, put where that slot's rows go.**
///
/// **A slot's rows are replaced whole rather than rewritten in place**, and
/// that is the change ADR-0326 made here. A row used to be one slot, so the
/// slot's row could be found and its three fields overwritten; a row is now
/// one node a build changed, so how many rows a slot has moves with every
/// build — two on the save that touched two files, one on the next, none on a
/// verdict with nothing to diff. Nothing is left behind: the newest verdict is
/// the whole of what this slot has to say, and a row from the build before it
/// would be a node reported as unsettled by a build that has been replaced.
///
/// **Slot order, because that is the order the rows are read in** — the lane
/// draws them top to bottom and the deck letters are `A` through `D`, so a slot
/// whose verdict arrived later must not sit above one whose arrived first. The
/// splice is over at most `deck::MAX_SLOTS` slots' worth of rows.
///
/// **And node order within a slot**, which is the order `changed` is in and is
/// the order the Set names its own nodes in — the geometries, the deformers,
/// the cameras, the renderers, the fields. It is the Inspector's pane order
/// read on one slot, so two bays list one slot's nodes the same way round.
pub(crate) fn settle(
    out: &mut Vec<view::Candidate>,
    deck: usize,
    label: &str,
    stage: view::Stage,
    // **What the checker said**, and empty for every verdict that is about a
    // build — see `view::Candidate::said`. Cloned rather than moved because
    // the event is read through `verdict`, which borrows it for the label;
    // it is a handful of short lines on the frame a refusal arrives, which is
    // a frame on which no Set was built.
    said: &[String],
    // **The nodes this build changed**, and empty for a verdict that has none
    // — see [`Changed`] and [`changed_rows`]. Empty draws **one** row on the
    // slot with no address on it, which is the honest answer for a build that
    // did not happen and for a rebuild that restated the stack unchanged: the
    // verdict is about the slot and there is no node to pin it to.
    changed: &[Changed],
) {
    let at = out.partition_point(|row| row.deck < deck);
    let end = at + out[at..].partition_point(|row| row.deck == deck);
    let rows: Vec<view::Candidate> = match changed.is_empty() {
        true => vec![view::Candidate {
            deck,
            at: None,
            addr: String::new(),
            name: label.to_owned(),
            stage,
            said: said.to_vec(),
        }],
        false => changed
            .iter()
            .map(|node| view::Candidate {
                deck,
                at: Some(node.at),
                addr: node.addr.clone(),
                name: node.name.clone(),
                stage,
                said: said.to_vec(),
            })
            .collect(),
    };
    out.splice(at..end, rows);
}

/// **The layer half of a node's address, as the mock's `.addr` spells it** —
/// `L1:0`, `L2:0`, `L4`.
///
/// `karakuri_ir::Kind` carries no name of its own, and `karakuri-cli`'s
/// `--publish name=L4:0:key` parser is in a package with no library target, so
/// there is nothing to call. The five words are `docs/ir-spec.md`'s and this
/// is a **match** for [`blend_mode`]'s reason: a sixth kind stops the build
/// here rather than drawing an address nothing can be typed back in.
pub(crate) fn layer_word(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
        Layer::Field => "F",
        // **A bare letter like the four above and unlike `F`'s neighbour**,
        // which is the address a press types back in — `L5:0`, on
        // `karakuri_environment::setfile::layer_ordinal`'s numbering.
        Layer::L5 => "L5",
    }
}

/// **A node's layer as the vocabulary names one.** `karakuri_ir::Kind` and
/// `karakuri_operation::Layer` are two spellings of one list, and this is the
/// converter between them; `mcp.rs`'s `layer_of` is the other instance and is
/// private to that crate. A match rather than a cast, for [`layer_word`]'s
/// reason: a sixth kind stops the build here.
pub(crate) fn asked_layer(layer: Layer) -> karakuri_operation::Layer {
    match layer {
        Layer::L1 => karakuri_operation::Layer::L1,
        Layer::L2 => karakuri_operation::Layer::L2,
        Layer::L3 => karakuri_operation::Layer::L3,
        Layer::L4 => karakuri_operation::Layer::L4,
        Layer::Field => karakuri_operation::Layer::Field,
        Layer::L5 => karakuri_operation::Layer::L5,
    }
}

/// **And back**, for the two things that want the compiler's own: spelling an
/// address a press arrived with, and resolving one to the word the store files
/// a version under. `karakuri_mcp`'s `kind_of` is the other
/// instance and is private to that crate; this is [`asked_layer`]'s inverse
/// and a match for its reason, so a sixth layer stops the build in both
/// directions rather than in one.
pub(crate) fn ir_layer(layer: karakuri_operation::Layer) -> Layer {
    match layer {
        karakuri_operation::Layer::L1 => Layer::L1,
        karakuri_operation::Layer::L2 => Layer::L2,
        karakuri_operation::Layer::L3 => Layer::L3,
        karakuri_operation::Layer::L4 => Layer::L4,
        karakuri_operation::Layer::Field => Layer::Field,
        karakuri_operation::Layer::L5 => Layer::L5,
    }
}

/// **Which node a published control belongs to**, and `None` where it belongs
/// to no one node.
///
/// `Published::at` is `Some` for a control an author addressed — the
/// `--publish name=L4:0:exposure` form — and `None` for a **wildcard**, which
/// is what the whole of the *default* interface is made of: *"one control per
/// key, not one per declaration"*, covering every node that declares the key.
/// This program has no `--publish` flag, so every control it ever draws is a
/// wildcard.
///
/// **A wildcard over exactly one node is that node's**, and the resolution
/// invents nothing: *where a bare name lands* is a set the Set itself
/// determines, and where it holds one member there is no second group the row
/// could go in. Over two or more it belongs to several groups at once, and the
/// mock draws no `.param` outside a `.node-group` — so the row is dropped and
/// [`inspector`] says how many were, rather than a place for it being invented
/// here (ADR-0200: *no placeholder, and no empty case the mock did not itself
/// draw*).
///
/// **The landing is asked for rather than worked out here**, and that is a
/// correction rather than a tidying. This walked `Set::params` and counted the
/// nodes holding the key, which is the same walk `Set::write_param` refuses on
/// — agreeing with it by coincidence. The day the built-in camera declared a
/// `radius` of its own the two stopped agreeing: a bare name does not reach
/// that node, so the engine saw one landing where this saw two, and a `radius`
/// row the pane draws every run was dropped as belonging to several groups
/// (ADR-0318). `Set::landing_of` is the one answer, where the write is decided
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
pub(crate) fn node_of(set: &Set, control: &Published) -> Option<(Layer, u32)> {
    if let Some(at) = control.at {
        return Some(at);
    }
    let mut declaring = set.landing_of(&control.key).into_iter();
    let first = declaring.next()?;
    match declaring.next() {
        None => Some(first),
        Some(_) => None,
    }
}

/// **What is driving one node's parameter**, or `None` where nothing is —
/// the reading behind a `.param.bound` row and the `.sens` row under it.
///
/// # The first match, because that is what the engine writes
///
/// `Set::bindings` is a list and `karakuri_engine::set::effective` takes the
/// **first** entry matching the layer, the key and the node, so a pane that
/// took the last would draw a source that is not the one holding the control.
/// This is that same `find`, and it is the one place in this file that reads a
/// binding at all: the pane's seventh reading, which ADR-0191 kept out while
/// nothing in this program could attach one.
///
/// **Addressed by the node the row was resolved to**, which is safe here for
/// the reason it would not be safe for a *write*: every row this pane draws
/// covers exactly one node — [`node_of`] drops the ones that do not — so
/// asking which binding covers that node is asking about the row itself. The
/// address the take-back and the re-attach carry is the **binding's** own, off
/// the entry found, and not this pair (ADR-0286: the placement is where a row
/// goes, the address is what a control is).
pub(crate) fn source_of(set: &Set, layer: Layer, index: u32, key: &str) -> Option<view::Source> {
    let binding = set
        .bindings()
        .iter()
        .find(|b| b.layer == layer && b.key == key && b.covers(index as usize))?;
    Some(view::Source {
        signal: binding.signal.clone(),
        curve: mix::curve(binding.curve),
        range: binding.range,
        at: karakuri_operation::BindAt {
            layer: asked_layer(binding.layer),
            index: binding.index,
            key: binding.key.clone(),
        },
    })
}

/// **What the Inspector's panes read**: one pane per slot the deck has, up to
/// the [`PANES`] the arrangement has, out of the seven things a `Set` will say
/// about itself.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one —
/// `slot`, `transport` and the `Set` behind each slot are its own — and what
/// crosses into the console is a word, some numbers and some names (ADR-0156).
///
/// # Where each one comes from, and what is checked before it is drawn
///
/// All seven reads answer off a running `Set`, and each was checked against
/// the source rather than taken on trust:
///
/// - **`Set::layering`** is the fold chip. It is a *build* decision —
///   `merge.is_some()` — so it is a readout here and the mock's press is a
///   rebuild rather than a write.
/// - **`Set::inputs`** is the renderer row: one edge per renderer in draw
///   order, and `Input::live` says which one reaches the screen. It is
///   *"empty of meaning under `Layering::Overdraw`"* by the engine's own
///   words, so `live` is only ever passed on under composite — under overdraw
///   every renderer draws and marking one would assert a choice the layering
///   does not make.
/// - **`Set::node_names`** is a name per node, in node order, and
///   `Set::node_named` turns each one back into the `(layer, index)` the mock's
///   `.addr` is.
/// - **`Set::authority`** is the `man / sug / auto` chip, through
///   [`mix::authority`], and the node's own address goes across beside it
///   because a press on a chip has to say which node it is about
///   (`view::NodeAuthority`). **A press writes one now**: `Deck::set_authority`
///   landed with ADR-0319, so the chips are three destinations rather than a
///   drawing. What a run *starts* at is still the default on every node —
///   `Request::authorities` is empty, because `Record::Authority` is
///   deliberately excluded from Set-file state in two places (ADR-0216) — and
///   that is the default being read rather than a placeholder being drawn: a
///   node nobody has spoken for **is** manual.
/// - **`Set::published`** is which controls appear and in what order, which is
///   what numbers the rows. It **allocates and says it is not for the frame
///   path**, which is why this is called once — see below.
/// - **`Set::params`** is what resolves a wildcard control to a node — see
///   [`node_of`].
/// - **`Set::bindings`** is the seventh, and it was the one this did **not**
///   read until 2026-09-09. The reason was ADR-0191's — nothing in this
///   program bound a signal to anything, so it was empty in every run and a
///   `.pval.src` drawn off it would have been a state the engine never
///   entered — and what changed is that the sensitivity row's chips can attach
///   one (ADR-0319). [`source_of`] is the read, at the node each row was
///   resolved to, and it takes the **first** matching entry because that is
///   what `karakuri_engine::set::effective` writes.
///
/// # Read once, and that is the engine's instruction rather than a shortcut
///
/// `Set::published` is documented *"Allocates, so not the frame path. A
/// console reads this when a Set lands, not per frame."* **A Set lands
/// whenever a `.kir` in a slot is saved**, since every slot is watched, so
/// this is called at startup and again on the frame a build is installed —
/// `staging` is what answers *did one land*, and it is the only thing in this
/// program that knows. Between those
/// frames almost every value above used to be constant. **Four things move
/// one now**, and each is a press: a `ride`, a `source`, an `authority` and a
/// `transport`. Every one of them re-reads the panes from
/// [`Readout::performed`], off the *record* rather than off the operation, so
/// a second operation writing one is caught by the same line — and none of
/// them is on a frame.
///
/// **The transport was the first that moved, and it is why this is called a
/// second time.** It had no caller outside its own tests when this was written
/// (ADR-0218); the deck head's scrub is that caller, so a press that writes a
/// `Record::Transport` re-reads the panes in [`Readout::performed`] — on the
/// press, which is where a directory read already happens, and never on a
/// frame. Re-reading the whole pane rather than writing the two numbers back
/// is deliberate: a second writer into `View::inspector` is a second answer to
/// *what is this pane showing*, and the reading that draws the anchor has to
/// be the reading the deck holds.
///
/// **The other three arrived with ADR-0319 and ADR-0280**, which is what that
/// sentence was waiting for: the writer the authority chip wanted is
/// `Deck::set_authority`, and the parameter's is `Deck::write_param`.
pub(crate) fn inspector(
    deck: &Deck,
    names: &[String],
    // **Where each slot's watcher is pointed**, which is what a node's `uses`
    // line reads: `Aiming::at.edges` is the wiring that slot was last sent, so
    // a pane draws the wiring of the deck it is *showing* rather than a run
    // list read once for four decks. It is the same list — `rewired` restates
    // the run's whole wiring to every slot it names — and this is the copy that
    // belongs to the slot the pane is about.
    //
    // **A pane with no aim behind it draws no `uses` line**, which is every
    // test in this crate that hands none in and is honest either way: a slot
    // nothing is pointed at is a slot nothing will rebuild.
    aims: &[Aiming],
    // **Which deck each pane is pointed at**, which is the console's own
    // pointer read back — `view::View::pane_deck`, one per pane, moved by the
    // pulldown on that pane's head (ADR-0338, decision 5).
    //
    // **It is handed in rather than read here**, which is `View::target_deck`'s
    // arrangement one bay along: the pointer belongs to the console and what
    // is under it belongs to the deck, so this function answers *what is that
    // slot playing* and never *which slot should this pane show*.
    //
    // Panes used to be filled slot by slot — pane `n` from slot `n` — so slots
    // C and D had no way onto this bay at all. That arrangement is the default
    // this array opens with (`view::PANE_DECKS`) rather than a rule.
    targets: [u8; view::PANES],
    out: &mut Vec<view::Pane>,
) {
    out.clear();
    // **A pane per target, in pane order**, which is what makes the position
    // of an entry in `out` the pane it belongs to — `view::inspector` reads it
    // by index, and `View::inspector` is walked the same way.
    for target in targets {
        let slot = usize::from(target);
        // **A pane pointed at a deck this run has no slot for draws nothing**,
        // and the panes after it go with it because a pane is addressed by its
        // position in this list. `View::point_pane` refuses a deck the mixer
        // draws no strip for, so the only way here is the tail of the default:
        // a run with one slot opens with the second pane pointed at deck B,
        // which is where `slot_count().min(PANES)` left it before this
        // pointer existed.
        if slot >= deck.slot_count() {
            break;
        }
        // The strip's name for the same slot, and for [`mixer`]'s reason: a
        // pane head reads `deck A · drift_night`, and after a load that is the
        // Set the operator chose rather than the pair the run opened with.
        let material = names.get(slot).map_or("", String::as_str);
        let addr = EngineSlot(target);
        let set = deck.slot(addr).set();
        let transport = deck.transport(addr);
        let composite = set.layering() == Layering::Composite;
        // **The deck head's two build chips**, and all three readings are of
        // what **landed** rather than of what was asked: the number the slot is
        // running at, the declaration it is measured against, and the salt the
        // next one is derived from. That is the fold's own division one field
        // over — a build may still be rolled back, and the Staging lane is what
        // says so — and it is what lets this be read off a `Set` with no aim in
        // sight (ADR-0328).
        let declared = set.declared_capacities();
        let running = set.source_capacities();
        let salts = set.source_salts();
        let aimed = match (running.first(), declared.first(), salts.first()) {
            (Some(&capacity), Some(&[_, _, default]), Some(&salt)) => Some(view::Aimed {
                capacity,
                // **Lit says the deck is not on what its material declares**,
                // which is the fact a chip can state from what landed. *An aim
                // carries a number* is the other candidate and is a reading of
                // what was asked: a hand that steps round to the declared
                // default would leave the chip lit over a slot running exactly
                // what its files say.
                stated: capacity != default,
                capacities: capacity_ladder(declared),
                salt: karakuri_engine::set::derived_salt(salt, 1),
            }),
            // **A deck with no geometry has neither chip**, which is the state
            // this `Option` is: there is no element count to size and no
            // randomness to seed.
            _ => None,
        };

        // Every published control, resolved to the node it belongs to and
        // numbered by its position in the interface — which is the number a
        // MIDI control is learned against, so it counts the controls that were
        // published and not the rows that could be placed.
        let published = set.published();
        // **And every control the material declares**, which is what the
        // publish mark is chosen *from*: a row taken off the interface has to
        // stay drawn or the choice cannot be unmade, and its declared range is
        // what putting it back is over. `Set::declared_interface` is the
        // reading — the default interface, whether or not one is authored
        // (ADR-0100, `docs/adr/0329-…`).
        //
        // **Identity is the address and the key together**, which is
        // `Published`'s own pair: a wildcard control and an addressed one may
        // share a key and are two controls, and comparing names would fold a
        // renamed control onto the declaration it renames.
        let declared = set.declared_interface();
        let mut rows: Vec<(Option<(Layer, u32)>, view::Param)> = Vec::new();
        for (at, control) in published.iter().enumerate() {
            // **By the address the control carries, not by its name.** The
            // built-in camera's three publish addressed, so a Set whose
            // geometry also declares `radius` has two controls under that name
            // and a name lookup answers for whichever comes first (ADR-0318).
            let value = set
                .value_at(control.at, &control.key)
                .unwrap_or(control.range[0]);
            let node = node_of(set, control);
            rows.push((
                node,
                view::Param {
                    ord: Some(at + 1),
                    name: control.name.clone(),
                    value,
                    // **The range and not the position.** A fader a hand can
                    // move has a second reader — the grab — so the map from a
                    // value to a place on the track is one statement in one
                    // place, `view::Param::at` and `view::Param::valued`, and
                    // the guard on a range of no width went with it
                    // (ADR-0286).
                    range: control.range,
                    // **The control the interface published, and not the
                    // group `node_of` put the row in.** A wildcard stays a
                    // wildcard, so a bare name goes on meaning every node that
                    // declares the key and meets `Set::write_param`'s
                    // authority refusal (ADR-0286, ADR-0223).
                    param: karakuri_operation::ParamAt {
                        node: control
                            .at
                            .map(|(layer, index)| karakuri_operation::NodeAddress {
                                layer: asked_layer(layer),
                                index,
                            }),
                        key: control.key.clone(),
                    },
                    // **The seventh reading, and the one this pane used to
                    // leave out.** It was omitted on ADR-0191's terms —
                    // nothing in this program bound anything, so a bound row
                    // was a state the engine could not enter — and what
                    // changed is that a press can attach one now (ADR-0319).
                    // Asked at the node the row was resolved to, which is the
                    // node the row *is*: `node_of` drops a control covering
                    // more than one.
                    bound: node
                        .and_then(|(layer, index)| source_of(set, layer, index, &control.key)),
                },
            ));
        }
        // **Then every declared control the interface leaves out**, drawn with
        // no number, no fader and no figure — the mark that publishes them is
        // the cell the number would be in, so a row that vanished would be a
        // choice nobody could unmake (`docs/adr/0329-…`).
        //
        // **After the published ones**, which is the order they are drawn in
        // *within a group*: a group's published rows come first and the ones
        // off the list follow, so the numbers a reader is counting down do not
        // step over a gap.
        //
        // **On a Set nobody has narrowed this loop adds nothing**, because
        // `Set::published` answers with `declared_interface` itself there —
        // which is why every deck opens looking exactly as it did before this
        // control existed.
        let mut unplaced = 0;
        for control in declared {
            if published
                .iter()
                .any(|shown| shown.at == control.at && shown.key == control.key)
            {
                continue;
            }
            unplaced += 1;
            let node = node_of(set, &control);
            rows.push((
                node,
                view::Param {
                    // **No position, because a control off the interface has
                    // none** — and a position is what a MIDI knob counts.
                    ord: None,
                    name: control.name.clone(),
                    value: set
                        .value_at(control.at, &control.key)
                        .unwrap_or(control.range[0]),
                    // **The declared range and not a narrowed one**: this is
                    // what publishing it back would be over, and the row is
                    // drawn from `declared_interface`, which never narrows.
                    range: control.range,
                    param: karakuri_operation::ParamAt {
                        node: control
                            .at
                            .map(|(layer, index)| karakuri_operation::NodeAddress {
                                layer: asked_layer(layer),
                                index,
                            }),
                        key: control.key.clone(),
                    },
                    // **Nothing can be holding it**, because a binding names a
                    // published control or a param and the row draws neither a
                    // value nor a sensitivity row. Read anyway rather than
                    // assumed `None`: what a Set holds is the engine's answer
                    // and this file does not have a second one.
                    bound: node
                        .and_then(|(layer, index)| source_of(set, layer, index, &control.key)),
                },
            ));
        }

        // **Which L3 node is the built-in camera**, which is the one node on a
        // pane with no procedure behind it and so nothing to keep.
        //
        // **The last camera node, always** — `Set::cameras`: *"Never empty,
        // and the last one is always the built-in orbit."* A Set whose files
        // declare no `kind L3` holds it at `L3:0`, and one that declares two
        // holds it at `L3:2`; either way it is the highest index on that
        // layer, so this is a `max` rather than a check for an empty layer.
        //
        // **Asked here rather than of the store**, because what a `keep` needs
        // is *is there a source at all*, and the Set is what knows. The bytes
        // themselves are the host's to find at the press — `Keeping::playing`
        // — and a pane that named a node the store cannot answer for would be
        // a capsule refusing after it was drawn.
        let builtin_camera = set
            .node_names()
            .iter()
            .filter_map(|name| set.node_named(name))
            .filter(|(layer, _)| *layer == Layer::L3)
            .map(|(_, index)| index)
            .max();
        let mut nodes: Vec<view::Node> = Vec::new();
        let mut renderers: Vec<view::Renderer> = Vec::new();
        let mut renderer_nodes = 0;
        let mut renderer_authority = None;
        let mut renderer_keep = None;
        for name in set.node_names() {
            let Some((layer, index)) = set.node_named(name) else {
                continue;
            };
            // **The address goes with the level**, because a press on a chip
            // has to say which node it is about and the two are absent
            // together — `view::NodeAuthority`, which is why this is one field
            // over there rather than two.
            let authority = set
                .authority(layer, index)
                .map(|level| view::NodeAuthority {
                    at: karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    },
                    level: mix::authority(level),
                });
            let params = |layer: Layer, index: u32| {
                rows.iter()
                    .filter(|(at, _)| *at == Some((layer, index)))
                    .map(|(_, param)| param.clone())
                    .collect::<Vec<_>>()
            };
            if layer == Layer::L4 {
                // **The renderers fold into one group**, which is the mock's
                // own `L4 renderers` head over a row of chips: the chips *are*
                // the L4 nodes, and the row is what the fold turns into a
                // choice. Every other layer is one group per node, addressed
                // `L2:0` the way the mock addresses it.
                renderers.push(view::Renderer {
                    name: name.clone(),
                    live: composite
                        && set
                            .inputs()
                            .get(index as usize)
                            .is_some_and(|edge| edge.live),
                });
                renderer_nodes += 1;
                renderer_authority = match renderer_nodes {
                    1 => authority,
                    // **More than one node under one head has no one
                    // authority**, and authority is per node (ADR-0216). The
                    // chip is dropped rather than showing the first of them.
                    _ => None,
                };
                // **And no capsule either, for that sentence** — one `keep` on
                // a head standing over three renderers would keep one of the
                // three and say nothing about which (ADR-0338, decision 4).
                // Open the fold and each renderer has its own; a Set with one
                // renderer has one node under that head and carries the
                // capsule like any other.
                renderer_keep = match renderer_nodes {
                    1 => Some(karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    }),
                    _ => None,
                };
                continue;
            }
            nodes.push(view::Node {
                addr: node_addr(layer, index),
                name: name.clone(),
                authority,
                // **Every node but the built-in camera has a source to keep**,
                // which is what `builtin_camera` above answers. The address
                // rides with it rather than beside it, for `view::Node::keep`'s
                // own reason: a head with nothing to keep has no node either.
                keep: (layer != Layer::L3 || Some(index) != builtin_camera).then_some(
                    karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    },
                ),
                renderers: Vec::new(),
                uses: match aims.get(slot) {
                    Some(aiming) => uses_of(set, &aiming.at.edges, name),
                    None => Vec::new(),
                },
                params: params(layer, index),
            });
        }
        if renderer_nodes > 0 {
            // The mock's address for the folded head is the bare layer, with
            // no index — because it is not one node's.
            let mut params: Vec<view::Param> = Vec::new();
            for index in 0..renderer_nodes {
                params.extend(
                    rows.iter()
                        .filter(|(at, _)| *at == Some((Layer::L4, index)))
                        .map(|(_, param)| param.clone()),
                );
            }
            // **Published first and in interface order, then the ones off the
            // list.** `Option`'s ordering puts `None` last, which is the order
            // the rows are drawn in within a group and is the reason the sort
            // is on the whole field rather than on a position: a number a
            // reader is counting down should not step over a gap.
            params.sort_by_key(|param| param.ord);
            nodes.push(view::Node {
                addr: layer_word(Layer::L4).to_owned(),
                name: RENDERERS_NODE.to_owned(),
                authority: renderer_authority,
                keep: renderer_keep,
                renderers,
                // **The folded renderer head takes none**, and it is the same
                // reason its authority chip is dropped where it stands over
                // more than one node: a `uses` line names *one* node's
                // declaration, and this head is not a node. A renderer that
                // declares an input is reachable from the file and from a
                // model, and the panel says so rather than drawing one of
                // several answers as the answer (ADR-0216's shape).
                uses: Vec::new(),
                params,
            });
        }

        let placed: usize = nodes.iter().map(|node| node.params.len()).sum();
        if placed < published.len() + unplaced {
            // **Said rather than swallowed**, for the reason every other
            // omission in this file is said: a pane short of a row looks
            // exactly like a Set that published fewer. See `node_of` — a
            // wildcard over two or more nodes belongs to two or more groups,
            // and the mock draws no row outside one.
            println!(
                "inspector: deck {} publishes {} controls and {} of them name no one node, so \
                 they have no group to sit in and are not drawn",
                DECK_LETTERS.get(slot).copied().unwrap_or("?"),
                published.len(),
                published.len() - placed
            );
        }

        out.push(view::Pane {
            deck: slot,
            material: material.to_owned(),
            sync: mix::sync(transport.sync()),
            // **What the sync chip's cycle skips over, asked of the engine
            // three times.** `Deck::sync_allowed` is *"what a surface greys a
            // control out on, and it answers before anything is pressed"* —
            // whether the Set in this slot is closed form and whether it reads
            // `beats`, put through `Transport::allows`. The console is handed
            // the three answers rather than the two properties, because what
            // may be asked for is not a surface's to work out (P-0090) and a
            // third copy of that rule in a crate with no material in it is a
            // rule that can start disagreeing.
            //
            // **`EngineSync::ALL` is in `SYNCS`' order**, which is what makes
            // this array line up with the field it fills;
            // `the_two_crates_walk_the_sync_modes_in_one_order` is what says
            // so rather than this comment.
            allows: EngineSync::ALL.map(|mode| deck.sync_allowed(addr, mode).is_ok()),
            anchor_bpm: transport.anchor_bpm(),
            scrub_beats: transport.scrub_beats(),
            composite,
            aimed,
            nodes,
        });
    }
}

/// **The `uses` lines one node draws**: every input its procedure declares,
/// with the node filling each.
///
/// # The edges *are* the declarations, and that is forced rather than chosen
///
/// Nothing on a built `Set` says which inputs a node declares — the `uses`
/// declaration is read at `Set::validate` and dropped — and nothing has to,
/// because **an unbound declared input is refused where the Set is built**
/// (ADR-0152: *"`If there is exactly one, use it` is the implicit rule the whole
/// item exists to remove, and the refusal names the slot"*). So a slot that is
/// *running* has an edge for every input it declares, and the run's edge list
/// filtered to the nodes this Set holds is that list exactly. A reader on the
/// engine would be a second answer to a question the refusal already settles.
///
/// # The candidates are the nodes on the layer the input already reaches
///
/// A `uses` slot has a type — `Geometry`, `Field`, `Camera`, `Source` — and the
/// build refused anything else, so **the node currently wired is of the right
/// kind by construction** and its layer is the kind. The candidates are the
/// other nodes on that layer, in node order, which is a list every entry of
/// which the build accepts.
///
/// **It is inference and it is honest about being it.** What this cannot do is
/// offer a kind the input takes and the deck currently reaches by no edge — a
/// `Field` input on a deck holding one field has an empty list, and the card
/// does not open. That is a control offering less than the language allows
/// rather than more, which is the side of P-0090 to be wrong on: a name this
/// misses is still reachable from a model and from `--edge`.
///
/// **The declaring node is not in its own list.** A node wired to itself is a
/// cycle the build refuses, and offering it would be offering a refusal.
pub(crate) fn uses_of(
    set: &karakuri_engine::set::Set,
    edges: &[karakuri_engine::set::Edge],
    node: &str,
) -> Vec<view::Uses> {
    edges
        .iter()
        .filter(|edge| edge.node == node)
        .filter_map(|edge| {
            let (kind, _) = set.node_named(&edge.to)?;
            let candidates = set
                .node_names()
                .iter()
                .filter(|name| name.as_str() != node && name.as_str() != edge.to)
                .filter(|name| set.node_named(name).is_some_and(|(at, _)| at == kind))
                .cloned()
                .collect();
            Some(view::Uses {
                slot: edge.slot.as_str().into(),
                to: edge.to.clone(),
                candidates,
            })
        })
        .collect()
}

/// **What a slot's capacity chip steps through**: the powers of two every one
/// of this deck's geometries would accept, ascending.
///
/// # The intersection, because a re-aim sends one number
///
/// `watch::Aim::capacity` is one `Option<u32>` for the whole slot —
/// `--capacity`'s own field, which *"overrides every source"* — so a Set
/// holding two geometries builds both at whatever this asks for, and a number
/// only one of them declares is a build the other refuses. The fold is
/// `lo.max(min)`, `hi.min(max)`, which is `declared`'s arithmetic one bay over
/// where two nodes publish one key: the range is the part every declarer
/// accepts and never any one of them on its own.
///
/// **An empty intersection is an empty list**, and that is a real state rather
/// than an unreachable one: two geometries whose declared ranges do not overlap
/// have no capacity a single re-aim could send. The chip is then drawn and
/// claims nothing, which is what `view::Aimed::capacities` says at the field.
///
/// **Powers of two, and nothing here says why they are the right rungs** — that
/// is the console's affordance and its record
/// (`docs/adr/0328-…`); what this owes is that every rung it offers is one the
/// engine will build, which is `Set::declared_capacities` being the same
/// declaration `karakuri_engine::set::capacity_in_range` refuses against
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
pub(crate) fn capacity_ladder(declared: &[[u32; 3]]) -> Vec<u32> {
    let Some(lo) = declared.iter().map(|at| at[0]).max() else {
        return Vec::new();
    };
    let Some(hi) = declared.iter().map(|at| at[1]).min() else {
        return Vec::new();
    };
    // `0..32` and not `0..=32`: `1u32 << 32` is undefined, and 2^31 is the
    // largest power of two a `u32` capacity can be.
    (0..32)
        .map(|k| 1u32 << k)
        .filter(|rung| (lo..=hi).contains(rung))
        .collect()
}

/// **What the mock calls the group its renderer chips sit under.** Not a node
/// name — every L4 node has one of those and they are the chips themselves —
/// but the head over all of them, which the mock writes as `L4 renderers`.
pub(crate) const RENDERERS_NODE: &str = "renderers";

/// **The engine's residency, as the console's word for it** — and, like
/// [`blend_mode`], a `match` so that a fourth `Residency` stops the build here
/// rather than drawing a chip nothing can read.
///
/// One function for both halves of the pair. It was written inline for the
/// effective residency alone; the request needs exactly the same three arms,
/// and a second copy of them is a translation that can start disagreeing with
/// itself about what `Priming` is called.
pub(crate) fn tally(residency: Residency) -> view::Tally {
    match residency {
        Residency::Live => view::Tally::Live,
        Residency::Priming => view::Tally::Priming,
        Residency::Allocated => view::Tally::Allocated,
    }
}

/// **The engine's blend mode, as the vocabulary's** — and the one place the
/// two lists are made to agree.
///
/// `karakuri-operation` owns its own copy of every list a destination is drawn
/// from, which is the cost P-0090 says the vocabulary pays: *"The two rules —
/// be engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place for this list, on the harness side of the seam — the
/// same side [`mixer`] reads a `Deck` from (ADR-0156).
///
/// **A match, so the day a fourth mode lands in `karakuri_engine::deck::Blend`
/// this stops compiling.** That is the whole reason `view::Strip::blend` is a
/// `BlendMode` and not the engine's word: a `&str` handed through would draw
/// the new mode's name on a chip, and the chip would cycle three ways past a
/// state no operation can name and no MIDI map can reach, with nothing saying
/// so. Failing here is the loud failure P-0094 asks for.
///
/// **It stays this program's, and that is now settled rather than pending.**
/// The console cannot depend on the engine (ADR-0156), and
/// `karakuri-operation-record` cannot either — it is the vocabulary and the
/// records and nothing else, by charter. So the two lists meet on the harness
/// side of the seam, wherever a harness holds both, and ADR-0180's *"one
/// `From` impl per list in `karakuri-cli`"* cannot be written at all: neither
/// [`Blend`] nor [`BlendMode`] is that package's, and the orphan rule refuses
/// it ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
pub(crate) fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Add => BlendMode::Add,
        Blend::Over => BlendMode::Over,
        Blend::Max => BlendMode::Max,
    }
}

// **The blend cycle is `karakuri_console::view::after` and is not here.**
//
// `after_blend` stood here until 2026-09-10 and said the same three modes in
// the same order as the console's own cycle, with a test each. The key that
// asked it was `m`; under the grammar `space` on an addressed blend chip asks
// the console, and the console has always had the cycle because the *chip*
// needs it (ADR-0187, P-0090 — a toggle is an affordance built over operations
// by whoever draws the control). **So there is one cycle where there were
// two**, and `karakuri-console/tests/blend.rs` is what holds it: it walks
// `BlendMode::ALL` through the chip and asserts the wrap, which is exactly
// what the test deleted beside this function asserted from the other side
// (ADR-0333).
//
// `blend_mode` stays, and is what `holding` reads the deck through: the
// engine's three and the vocabulary's three are two crates' words for the same
// states, and a `match` is where they are made to agree.

/// **One press of a gain key.** Linear and additive, because a fader is: the
/// same press means the same amount wherever the trim is standing, rather than
/// a proportion of wherever it happens to be.
///
/// **A tenth, because that is what the other keyboard steps by.**
/// `docs/manual/operations.html` names the keys and says nothing about how far
/// a press goes, so the size comes from `karakuri-cli`'s own `GAIN_STEP` —
/// which `docs/manual.md` documents as *"focused slot gain down / up"* — and
/// it is copied rather than shared because neither binary may depend on the
/// other (ADR-0214). A page that decides otherwise moves this constant.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// One press of an opacity key, and [`GAIN_STEP`]'s sentence one control
/// along: `karakuri-cli`'s `OPACITY_STEP`, which is the same tenth, and the
/// console's page is silent about this one too.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// **Which of the grammar's four keys a press is**, or `None` for a key that is
/// not one of them.
///
/// # Why the digits are a guard and not ten arms
///
/// `1` in the Mixer is deck A's strip and `1` in the Library is its first
/// row, so ten arms naming ten literals would say the digits are bound and
/// say nothing about what they reach. **The dispatch table is what says
/// that** — `karakuri_console::focus::BUILT` — so the digit is a guard here
/// rather than ten characters (ADR-0333).
///
/// **`key_column::bound` no longer reads this file's text at all** — since
/// the ten literal keys `window_event` binds outside this guard moved into
/// `KEY_BINDINGS`, there is nothing left in this file's source for a scan to
/// find that a declared fact could not say instead. What this function still
/// binds — `space`, `enter`, the four arrows and the digit — is declared in
/// `key_column::bound` alongside `KEY_BINDINGS`' own keys rather than
/// scanned for, the same as the digit always was.
pub(crate) fn grammar(key: &Key<&str>) -> Option<focus::Press> {
    match key {
        Key::Named(NamedKey::Space) => Some(focus::Press::Space),
        Key::Named(NamedKey::Enter) => Some(focus::Press::Enter),
        Key::Named(NamedKey::ArrowUp) => Some(focus::Press::Arrow(focus::Arrow::Up)),
        Key::Named(NamedKey::ArrowDown) => Some(focus::Press::Arrow(focus::Arrow::Down)),
        Key::Named(NamedKey::ArrowLeft) => Some(focus::Press::Arrow(focus::Arrow::Left)),
        Key::Named(NamedKey::ArrowRight) => Some(focus::Press::Arrow(focus::Arrow::Right)),
        Key::Character(text) => digit(text).map(focus::Press::Digit),
        _ => None,
    }
}

/// **One digit, `0` to `9`**, or `None` for anything else a `Key::Character`
/// can be.
///
/// A `Key::Character` is *text* and may be more than one character — a dead key
/// resolving, an IME committing a run — which is why the length is checked
/// rather than the first character taken. `char::to_digit` at radix ten accepts
/// the ASCII ten and nothing else, so a digit from another script is not one
/// here: the digits count what a bay drew and the number row is what a
/// performer finds without looking.
pub(crate) fn digit(text: &str) -> Option<usize> {
    let mut chars = text.chars();
    let one = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    one.to_digit(10).map(|digit| digit as usize)
}

/// **What the deck is holding on `at`**, for the three chips whose next state
/// the console names — or `None` where this deck has no slot there.
///
/// [`held`] is the guard, for its own reason: `Deck::blend` indexes its slots
/// and a panic reachable from an event handler aborts this process rather than
/// unwinding.
///
/// **Read at the press and never off `view::Strip`**, which is what the three
/// mix keys this replaces already said: a strip is this same reading copied
/// once a frame, and a scheduled fade landing between the frame and the press
/// would leave the cycle counting from a state the deck has left behind.
pub(crate) fn holding(deck: &Deck, at: u8) -> Option<focus::Held> {
    let slot = held(deck, at)?;
    Some(focus::Held {
        // **The residency the deck was last *asked* for**, which is what the
        // tally cycles from — `view::Mixer::tally`'s decision, and the one
        // that makes a parked slot's press a withdrawal rather than a
        // re-request.
        requested: tally(deck.requested_residency(slot)),
        blend: blend_mode(deck.blend(slot)),
        mask: masked(deck.mask(slot).kind()),
        // The angle the slot is already wearing, carried through unchanged
        // (ADR-0203).
        mask_angle: deck.mask(slot).angle(),
    })
}

/// **The engine's mask shape as the console's**, and the mirror image of
/// `karakuri_console::view::wipe_kind` on the way back out.
///
/// One function and two callers — [`mixer`] builds a strip from it every frame
/// and [`holding`] reads it at a press — because two copies of a three-arm
/// translation is exactly the shape that goes wrong the day a fourth shape
/// lands: a `match` with no wildcard stops the build in one place instead of
/// two.
pub(crate) fn masked(kind: MaskKind) -> view::Mask {
    match kind {
        MaskKind::None => view::Mask::None,
        MaskKind::Linear => view::Mask::Linear,
        MaskKind::Radial => view::Mask::Radial,
    }
}

/// **Where a press takes the trim it is standing on.**
///
/// [`offset_step`]'s shape one bay along, with the grammar's own word for a
/// direction in place of a letter: which way each press goes is a value this
/// file can be asked about without a window.
///
/// # What the page does not say, and where each answer comes from
///
/// The row names the keys and stops. So the size of a step and the destination
/// [`Step::Default`] names are `karakuri-cli`'s `'['`, `']'` and `'\\'` —
/// *"focused slot gain down / up / back to 1.0"* in `docs/manual.md` — taken
/// whole rather than invented here, because two keyboards that disagree about
/// how far one press goes is the one mistake an operator makes in the dark and
/// cannot see. **The letters were this keyboard's too until 2026-09-10**, and
/// what survived them is the arithmetic rather than the spelling.
///
/// # Floored and not ceilinged, and the clamp is the surface's
///
/// A negative gain would subtract one slot's light from another's, which is a
/// blend mode rather than a level; above 1.0 is ordinary, because the pipeline
/// is HDR
/// ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
/// It is clamped **here** rather than left to `Deck::set_gain` for
/// `karakuri-cli`'s `clamp_gain` reason: this decides what the *record* says,
/// so a session replays the value that took effect rather than one the engine
/// quietly corrected.
///
/// **So [`Step::Default`] is a destination and the other two are steps**, and
/// all three leave as the same absolute [`Operation::SetGain`] — an absolute
/// value can express every step and a step cannot express a setting.
///
/// **It took the letter and takes the step since 2026-09-10.** `[`, `]` and
/// `\` are unbound: the trim is reached by addressing it — `space` on the
/// Mixer's strip — and the arrows step it (ADR-0259, ADR-0333). The pair of
/// directions and the tenth between them are unchanged and are still
/// `karakuri-cli`'s, which is what the paragraphs above are about; what went is
/// the letter that named each one.
pub(crate) fn gain_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - GAIN_STEP,
        Step::Up => from + GAIN_STEP,
        Step::Default => 1.0,
    };
    asked.max(0.0)
}

/// **Where a press takes the fader it is standing on** — [`gain_key`]'s
/// function on the other control.
///
/// The pair and the tenth between them are `karakuri-cli`'s, for the reason
/// written at [`gain_key`]: the page names the pair and not the direction, and
/// `;` down and `'` up were the letters until 2026-09-10.
///
/// **The fader gains a default here and did not have one.** The trim's `\`
/// had no partner on this control, so `space` on an addressed fader is the
/// first way back to unity it has ever had — ADR-0259's *"on a level, the one
/// state worth naming is the value it was declared at"*, which is the clause
/// that record buys with an argument rather than finds.
///
/// **Held inside `[0, 1]` where the gain is only floored**, which is the
/// difference the vocabulary already draws between the two: opacity is a
/// proportion of a blend and there is no such thing as 1.4 of one, where gain
/// is a level into an HDR mix. The clamp is this surface's for [`gain_key`]'s
/// reason — it decides what the record says.
pub(crate) fn opacity_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// **Where a press takes the master out** — [`gain_key`]'s function one bay
/// down, and the three answers are the same three.
///
/// **A tenth, and the same tenth**: the trim, the fader and this are one
/// gesture on three controls, and a keyboard that stepped each of them by a
/// different amount would be three keyboards. **Held inside `[0, 1]` where
/// the gain is only floored**, which is [`opacity_key`]'s distinction met on
/// the level the whole programme leaves through: `Knob::Out` drags over
/// exactly that range, so a key and a hand can reach the same values and no
/// others.
///
/// **The default is unity**, which is where a run starts and what the row
/// reads before anybody has touched it.
pub(crate) fn out_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// **Where a press takes the exposure** — a **quarter stop**, which is the
/// step the console's own track was built for.
///
/// `karakuri_console::view::EXPOSURE_TRACK_W`'s documentation is where that
/// number comes from and it says the whole argument: *"one pixel a press …  a
/// pointer on this track can ask for any of the 48 positions along it and a
/// keyboard stepping a quarter stop at a time can ask for any of the 48 values
/// between the ends, so neither surface can reach a value the other cannot"*.
/// So the step is taken on the **track's** axis and converted back, rather
/// than as a multiplier written here — the two surfaces then land on the same
/// 48 values by construction.
///
/// **Clamped by the conversion rather than here**: `unit_of` holds a value
/// past either end at that end and `exposure_at` runs over `[0, 1]`, which is
/// where `--exposure 200` is allowed to be unclamped and a press is not.
///
/// **The default is 1.0**, which is the middle of the track and the level a
/// run starts at — ADR-0259's *"`space` returns it to 1.0 — today's `` ` ``"*.
pub(crate) fn exposure_key(step: Step, from: f32) -> f32 {
    let at = view::unit_of(from);
    match step {
        Step::Down => view::exposure_at(at - 1.0 / view::EXPOSURE_TRACK_W),
        Step::Up => view::exposure_at(at + 1.0 / view::EXPOSURE_TRACK_W),
        Step::Default => 1.0,
    }
}

/// **Where a press takes the latency offset** — five milliseconds, which is
/// the page's own step and the one `karakuri-cli`'s `o` and `p` use.
///
/// **The sign is the half that gets read wrong at two in the morning**, and
/// `docs/manual/console.html` says so: *"Negative and the picture waits for the
/// music, positive and it leads."* So `Step::Down` is the picture waiting, and
/// this function is where a test can ask which way each direction goes — a
/// pair wired the wrong way round reads correct and points backwards.
///
/// `o` and `p` were this keyboard's letters until 2026-09-10, and what
/// survived them is the step rather than the spelling: the constant is
/// `karakuri_environment::audio`'s, which is what the command line steps by,
/// and this program does not keep a second copy of it.
///
/// **Not clamped here**, which is the one place this differs from [`gain_key`]
/// and [`opacity_key`]: the offset's range is `karakuri_environment::audio`'s
/// and the session holds a press at the end of its travel and says so
/// ([`offset_said`]). A second clamp here would decide the same thing twice.
///
/// **The default is zero**, which is the value the offset is declared at: a
/// session nobody has nudged runs at no offset at all.
pub(crate) fn offset_key(step: Step, from: f32) -> f32 {
    match step {
        Step::Down => from - audio::LATENCY_OFFSET_STEP_MS,
        Step::Up => from + audio::LATENCY_OFFSET_STEP_MS,
        Step::Default => 0.0,
    }
}

/// **The slot a press or a record names, as an index this deck has**, or
/// `None` where it has not got one.
///
/// `Deck::gain` and `Deck::set_gain` index their slots, and a panic reachable
/// from an event handler aborts this process rather than unwinding (see the
/// module documentation), so every route from a letter or a record to the deck
/// asks this first. `mix::change`'s whole reason for taking a `slot_count` is
/// that a stream may name a slot that is not there.
///
/// **Nothing in this file can produce one**: the strips are the deck's own
/// count, and `View::select` refuses a deck the mixer draws no strip for — so
/// this is the guard rather than the message, and the real sentence is
/// `karakuri-cli`'s `no_such_slot`.
///
/// **One derivation and not one per caller**, which is what makes the key arms
/// and [`apply`] refuse the same slot: a press reads the deck before it names
/// a destination and the record writes it afterwards, and a guard on only the
/// second of the two would be a read that panicked on its way to a refusal.
pub(crate) fn held(deck: &Deck, slot: u8) -> Option<EngineSlot> {
    EngineSlot::new(slot, deck.slot_count())
}

/// **The engine's look, as the console reads it** — [`blend_mode`]'s function
/// one row up, on the value every sink is drawn under.
///
/// Two fields of three: `white_point` is Reinhard's parameter, it is on no
/// surface, and a console field for it would be a reading no control names —
/// see `karakuri_console::view::Look`. The operator goes through
/// [`mix::tonemap`], which is the match that makes the engine's list and the
/// vocabulary's agree and stops compiling the day a fifth operator lands on
/// one side only.
pub(crate) fn look(look: &Look) -> view::Look {
    view::Look {
        tonemap: mix::tonemap(look.op),
        exposure: look.exposure,
    }
}

/// **What this window says when a control's operation wrote no record**, and
/// the two ways that happens are not the same thing — so they are not the
/// same sentence.
///
/// [`written`] has three answers and only one of them is a record.
/// A harness that printed a line for that one and nothing at all for the
/// other two would tell an operator that a press did nothing, which is true
/// of neither:
///
/// - [`Written::Silent`] is **settled**. Selecting a deck or folding a bay is
///   a surface's own state and there is nothing to write; the sentence says
///   which of the four kinds of nothing it is, and that is the end of it.
/// - [`Written::Owed`] is **a gap nobody has closed yet**. A tap owes a
///   record and no build can make it, so a press that reads as *nothing
///   happened* is exactly the wrong reading — the sentence names the question
///   instead, which is `Owed::why`'s whole job and the reason `Owed` is not
///   an error.
///
/// `None` for [`Written::Records`], because that line is [`apply`]'s: it says
/// the record *and* what the deck holds afterwards, and printing both would
/// say one press twice.
///
/// **Nothing on this panel reaches the `Owed` arm on purpose any more**, and
/// the paragraph that used to stand here is worth keeping as history because
/// it was twice wrong in the same place. It first said no control could reach
/// either arm and was written for the day one did; the deck head was that day,
/// and it said the sync chip and the anchor beside it were **reachable
/// affordances over an unwritable record** — the press claimed, the operation
/// emitted, this sentence printed with the question in it, and the deck not
/// moving.
///
/// **What made the record unwritable was a question that had already been
/// answered.** `Transport::engaged` decides what engaging a mode means, with
/// the reason at its own definition: the anchor is the session tempo and the
/// scrub is cleared. The clamp that looked like a decision about the bytes on
/// disk is the identity on every tempo an oscillator can report, so there were
/// never two answers to choose between — only a reading nobody was handing in.
/// [`reading`] hands it in now, `written` writes `Record::Transport`, and
/// [`apply`] moves the deck, which is the ninth and tenth of this panel's ten
/// emitting controls arriving where the other eight already were.
///
/// **The refusal to route around it is what made that cheap.** A surface owns
/// the affordance and never the authority
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// so this file never wrote a `Record::Transport` of its own for a `SetSync` —
/// computing the anchor here would have been a window binary taking a decision
/// about a file format, and the printed line was the right answer until the
/// conversion existed
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
/// What changed is the conversion, not this file's authority: the anchor is
/// still the engine's policy and this window still only reads a tempo.
///
/// So all ten controls write records. Five are the mixer's — `SetGain`,
/// `SetOpacity`, `SetBlendMode`, `SetResidency` and `SetMaskShape` — two are
/// the look's, and the last three are the deck head's: the scrub, the chip
/// that cycles and the anchor that re-asks for the mode the deck is in
/// (ADR-0218). The Outputs dot never arrives here at all, because it asks the
/// panel for an arrangement [`Op`] and the panel performs it
/// ([`Acted::Operated`]).
///
/// **The mask is also the one that can reach [`Written::Owed`] by accident**,
/// and that is worth having rather than designing away: a reading that did not
/// arrive answers `Owed(NotRead(Reading::Mask))`, so a harness that stopped
/// handing one in would say so out loud instead of moving nothing.
pub(crate) fn unwritten(operation: &Operation, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) => None,
        Written::Silent(silent) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is settled: {}",
            silent.why()
        )),
        Written::Owed(owed) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is a gap rather than a \
             decision: {}. nothing moved, and nothing here decides it",
            owed.why()
        )),
    }
}

/// **A press on a strip or a deck key, applied to the console's own pointer**,
/// and what to say about it. `None` for every operation that is not it.
///
/// `Operation::SelectDeck` *"writes no record, and is the reason every other
/// variant names its deck instead of meaning the selected one"*, so there is
/// nothing on the deck for [`apply`] to move and the surface that emits it is
/// what performs it (ADR-0198). This is that performance, and it is one line
/// beside [`arrangement`]'s for the same reason: the alternative is a second
/// route into the view.
///
/// **A deck the mixer has no strip for is refused**, and `View::select` is
/// where that rule lives — the ring would be drawn nowhere and the library's
/// pill would name a deck a load could not reach. It is said here rather than
/// swallowed, because a key that does nothing and a key that is not bound are
/// the same experience.
pub(crate) fn pointed(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SelectDeck { deck } = *operation else {
        return None;
    };
    let letter = deck_letter(deck);
    if usize::from(deck) >= view.mixer.len() {
        return Some(format!(
            "  select: deck {letter} refused — this deck has {} slot{}, and a selection with no \
             strip under it is a ring drawn nowhere and a `load` pill naming a deck the press \
             could not reach",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    view.select(deck);
    Some(format!(
        "  select: deck {letter} -> SelectDeck {{ deck: {deck} }} -> no record, and that is \
         settled: it is a surface's own pointer. The keys are addressed here, and the library's \
         foot reads `load -> {letter}`"
    ))
}

/// **A pick on a pane head's pulldown, applied to the console's own pointer**,
/// and what to say about it. `None` for every operation that is not it.
///
/// [`pointed`]'s shape one mark along and for its sentence:
/// `Operation::PointPane` is `Silent(Surface)`, so there is nothing on the deck
/// for [`apply`] to move and the surface that emits it performs it.
///
/// **It is not the deck selection**, and nothing here touches it — that is the
/// whole of what this mark is for (ADR-0338, decision 5): a pane can show a
/// deck the keys are not on, which is the Library bay's load pulldown's
/// argument one bay along.
///
/// **The pane is named rather than numbered**, because
/// `Operation::PointPane { pane }` is a `String` — `karakuri-operation` has no
/// dependencies and cannot hold the arrangement's handle type — so this is
/// where the name is resolved back to a position in `View::inspector`. A name
/// no pane has is refused with the two that exist, which is what the next
/// attempt needs (P-0083).
///
/// **A deck the mixer has no strip for is refused**, and `View::point_pane` is
/// where that rule lives — [`pointed`]'s own refusal, read on a pane instead of
/// on the ring.
pub(crate) fn pointed_pane(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::PointPane { pane, deck } = operation else {
        return None;
    };
    let Some(at) = view::PANE_NAMES.iter().position(|name| name == pane) else {
        return Some(format!(
            "  pane: `{pane}` refused — this console's panes are {}",
            view::PANE_NAMES.join(" and ")
        ));
    };
    let letter = deck_letter(*deck);
    if !view.point_pane(at, *deck) && usize::from(*deck) >= view.mixer.len() {
        return Some(format!(
            "  pane: `{pane}` -> deck {letter} refused — this deck has {} slot{}, and a pane \
             pointed at one it has not got is a head naming a deck with nothing under it",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    Some(format!(
        "  pane: `{pane}` -> deck {letter} -> no record, and that is settled: a pane's target is \
         a surface's own pointer. The keys stay where they are and the pane next door does not \
         move"
    ))
}

/// **What a refused `go` says**, and the whole of what this window puts on
/// this side of that seam.
///
/// `karakuri_console::view::Go` answers *which* refusal, because the console
/// is what can see a shape is unset and how many strips it drew; the sentence
/// is here because this package is the one that has anywhere to print. What
/// each of them owes is
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// a rejection is a message to whoever makes the next attempt, and it carries
/// the constraint and where to go rather than *refused*.
///
/// **`karakuri-cli`'s `c` prints the same two**, and its wording is where
/// these come from — *"a wipe needs somewhere to come from — this deck holds
/// one slot"* and *"no mask shape — `z` chooses one, and a wipe is a shape
/// moving"*. What changes on this surface is where the next attempt is made:
/// a pill two capsules to the left rather than a key.
pub(crate) fn refusal(refused: &Go, decks: usize) -> String {
    match refused {
        // **Unreachable while this window builds a full deck** — `SLOTS` is
        // `MAX_SLOTS` and the mixer draws one strip per slot — so what this
        // answers for is a console drawn before the deck reached it, and it
        // is written rather than left to be an unexplained silence. The count
        // is said because it is the thing that would have to change.
        Go::NoOtherDeck => format!(
            "  wipe: refused — this mixer draws {decks} strip{}, and a wipe needs a deck to \
             come from as well as one to arrive. Two decks side by side in the bay is what \
             makes the press mean something",
            match decks {
                1 => "",
                _ => "s",
            }
        ),
        Go::NoShape => String::from(
            "  wipe: refused — no shape chosen, and a wipe is a shape moving. The first pill \
             on this row is where one is picked: it reads `no shape` now, and a press on it \
             takes it to `left`. The vocabulary refuses this one too — \
             `Operation::Wipe` is *refused with no shape chosen* at its own definition — and \
             the refusal is here because the shape is this console's own setting",
        ),
        // A wipe is not a refusal, and this arm exists so that the day a
        // fourth answer lands somebody has to say what it reads rather than
        // a wildcard printing one of these two over it.
        Go::Wipe(operation) => format!(
            "  wipe: {} is not a refusal and this line should not have been reached",
            operation.title()
        ),
    }
}

/// **The four banks a run starts with**: bank 0 holding one lane over deck A's
/// channel fader, **muted**, and three empty banks beside it.
///
/// # Why there is a lane at all before anything has added one
///
/// `Operation::PointLane` is the control that makes a lane and **the console
/// draws none** — the mock's `+ lane` needs a target chooser nobody has drawn,
/// and the bay cannot grow a body while it is running anyway. So a first slice
/// with no lane would draw a ruler over nothing and there would be no way to
/// reach a cell (ADR-0320's consequences, and `view::sequencer`'s own list of
/// what is not drawn). This is the demonstration, written here rather than in
/// `karakuri-pattern` because it is *this program's opening state* and not
/// what a pattern is.
///
/// # Why it is muted
///
/// **An unmuted lane writes its target on every step boundary**, on-steps and
/// off-steps alike — that is what makes a lane a gate rather than a set of
/// impulses (ADR-0320). A lane over deck A's fader with every step off would
/// therefore hold deck A at `off` from the moment the window opened: the deck
/// on air would go dark, and nothing on screen would say a sequencer had done
/// it.
///
/// So the lane arrives the way the mock's third row is drawn — *"the pattern is
/// kept and drives nothing"* — and the first press is the one that starts it.
/// That is also the shape of the demonstration: mute the lane and the fader is
/// the hand's again, which is rule 02's take-back for a lane and what
/// ADR-0322 says the mute is *for*.
///
/// **`on` is 1.0 and `off` is 0.0**, which is a fader's pair: a gate.
pub(crate) fn demonstration_banks() -> karakuri_pattern::Banks {
    let mut banks = karakuri_pattern::Banks::default();
    let mut lane =
        karakuri_pattern::Lane::new(karakuri_operation::LaneTarget::Fader { deck: 0 }, 1.0, 0.0);
    lane.set_muted(true);
    if let Some(bank) = banks.at_mut(0) {
        bank.push(lane);
    }
    banks
}

/// **A lane appended to the bank the press named**, and what to say about it —
/// [`sequenced`]'s `PointLane` arm, lifted out because it is the one arm that
/// reads something other than the pattern.
///
/// # Where the two levels come from, and why they are not in the payload
///
/// A lane carries an `on` and an `off`
/// ([ADR-0320](../../docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md)),
/// filled in **at the press** and never read back later, because a pattern
/// outlives the Set it was written against. `Operation::PointLane` carries a
/// bank and a target and nothing else (ADR-0321), so they are filled here —
/// out of `View::inspector`, which is *the console's own published reading*
/// and the very list the chooser drew its items from.
///
/// **That is one reading and not two.** Asking the deck again here would be a
/// second derivation of the range, and the two could name different numbers
/// the frame a Set lands; the operator saw the console's, and the lane gets
/// the console's
/// ([ADR-0327](../../docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md)).
///
/// **A fader's are 1.0 and 0.0**, which is a gate and is that record's own
/// pair: a channel fader publishes no range, and `[0, 1]` is what the strip
/// draws.
///
/// **A parameter the console holds no row for is refused and said**, rather
/// than defaulted: a lane with invented levels would drive its target to two
/// numbers nobody chose, and the refusal names what the next attempt needs
/// ([P-0083](../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
pub(crate) fn pointed_lane(
    banks: &mut karakuri_pattern::Banks,
    view: &View,
    pattern: u8,
    target: &karakuri_operation::LaneTarget,
) -> String {
    let levels = match target {
        karakuri_operation::LaneTarget::Fader { .. } => Some((1.0, 0.0)),
        karakuri_operation::LaneTarget::Param { deck, param } => view
            .inspector
            .iter()
            .filter(|pane| pane.deck == usize::from(*deck))
            .flat_map(|pane| pane.nodes.iter())
            .flat_map(|node| node.params.iter())
            .find(|row| &row.param == param)
            .map(|row| (row.range[1], row.range[0])),
    };
    let Some((on, off)) = levels else {
        return format!(
            "  lane: pattern {pattern} is not pointed — this console holds no published range \
             for that control, and a lane's two levels are the range published to it at the \
             press. Point the Library bay's load pulldown at the deck whose control it is and \
             ask again"
        );
    };
    let Some(bank) = banks.at_mut(usize::from(pattern)) else {
        return format!("  lane: pattern {pattern} is not a bank this session holds");
    };
    // **It arrives muted, and that is not caution.** A lane arrives with every
    // slot off, and an off step writes `off` rather than writing nothing — so
    // an unmuted fader lane would hold its deck at zero from the press, which
    // is `demonstration_banks`' argument reached from the other end: the deck
    // would go dark and nothing on screen would say a sequencer had done it.
    // The label is the unmute and it is one press, which is rule 02's take-back
    // drawn where the lane is.
    let mut lane = karakuri_pattern::Lane::new(target.clone(), on, off);
    lane.set_muted(true);
    bank.push(lane);
    let lanes = bank.lanes().len();
    format!(
        "  lane: pattern {pattern} -> a {} lane, on {on} off {off} -> no record. {lanes} lane{} \
         under the rows, muted: every slot is off and an off step writes {off}, so the label is \
         the press that starts it",
        match target {
            karakuri_operation::LaneTarget::Fader { .. } => "fader",
            karakuri_operation::LaneTarget::Param { .. } => "parameter",
        },
        match lanes == 1 {
            true => "",
            false => "s",
        }
    )
}

/// **A press in the Sequencer bay, applied to the pattern it names**, and what
/// to say about it. `None` for every operation that is not one of the three.
///
/// [`scheduled`]'s shape one bay along, and for the same reason:
/// `written` answers `Silent(Surface)` for all five of the sequencer's
/// operations — a pattern is library data under the store on the arrangement's
/// terms, and what a *lane* does reaches the stream as its own writes
/// (ADR-0320, ADR-0322) — so there is nothing on the deck for [`apply`] to
/// move and the surface that emits it is what performs it.
///
/// **Every arm names its bank**, and a press on a bank this session does not
/// have is refused and said rather than swallowed, which is [`pointed`]'s rule:
/// a press that does nothing and a press that is not bound are the same
/// experience.
///
/// **A mode press and a bank press reset the playhead**, and a step press does
/// not. The first two change what a step *index* means — the same bar read at
/// another width, or another pattern's lanes under it — so a remembered index
/// would hold the new reading silent until the bar came round. Turning a cell
/// on changes what the *current* step is worth and not which step it is, and
/// the mock already says when that is heard: *"the next time the playhead
/// reaches the cell rather than when you asked for it"*.
pub(crate) fn sequenced(
    banks: &mut karakuri_pattern::Banks,
    playhead: &mut karakuri_pattern::Playhead,
    view: &View,
    operation: &Operation,
) -> Option<String> {
    if let Operation::PointLane { pattern, target } = operation {
        return Some(pointed_lane(banks, view, *pattern, target));
    }
    match *operation {
        Operation::SetStep {
            pattern,
            lane,
            step,
            on,
        } => {
            let Some(lane_at) = banks
                .at_mut(usize::from(pattern))
                .and_then(|at| at.lane_mut(usize::from(lane)))
            else {
                return Some(format!(
                    "  step: pattern {pattern} lane {lane} is not a lane this session holds — a \
                     press names a bank, a lane and a slot, and this one names no row"
                ));
            };
            lane_at.set_slot(usize::from(step), on);
            Some(format!(
                "  step: pattern {pattern} lane {lane} slot {step} -> {} -> no record, and that \
                 is settled: a pattern is library data and what a lane does is its own writes. \
                 Heard the next time the playhead reaches it",
                match on {
                    true => "on",
                    false => "off",
                }
            ))
        }
        Operation::SetLaneMute {
            pattern,
            lane,
            muted,
        } => {
            let Some(lane_at) = banks
                .at_mut(usize::from(pattern))
                .and_then(|at| at.lane_mut(usize::from(lane)))
            else {
                return Some(format!(
                    "  lane: pattern {pattern} lane {lane} is not a lane this session holds"
                ));
            };
            lane_at.set_muted(muted);
            Some(format!(
                "  lane: pattern {pattern} lane {lane} -> {} -> no record. {}",
                match muted {
                    true => "muted",
                    false => "driving",
                },
                match muted {
                    true =>
                        "The pattern is kept and drives nothing, and the fader is a hand's \
                             again — which is where this lane's take-back sits",
                    false => "It writes its target at the next step boundary",
                }
            ))
        }
        Operation::SetPatternGrid { pattern, grid } => {
            let Some(at) = banks.at_mut(usize::from(pattern)) else {
                return Some(format!(
                    "  grid: pattern {pattern} is not a bank this session holds"
                ));
            };
            at.set_mode(grid);
            // The same bar at another width, so the index it was remembering
            // is about a reading that has gone.
            playhead.reset();
            Some(format!(
                "  grid: pattern {pattern} -> {} -> no record. The bar is one bar, so the count \
                 follows: {} steps over the same row, and the sixteen slots underneath are \
                 untouched",
                grid.name(),
                grid.count()
            ))
        }
        Operation::SelectPattern { pattern } => {
            if !banks.select(usize::from(pattern)) {
                return Some(format!(
                    "  pattern: there is no bank {pattern} — this session holds {}",
                    karakuri_pattern::BANKS
                ));
            }
            playhead.reset();
            Some(format!(
                "  pattern: bank {pattern} armed -> no record. {} lane{} under the rows",
                banks.pattern().lanes().len(),
                match banks.pattern().lanes().len() == 1 {
                    true => "",
                    false => "s",
                }
            ))
        }
        _ => None,
    }
}

/// **A press on one of the transition row's three pills, applied to the
/// console's own setting**, and what to say about it. `None` for every
/// operation that is not it.
///
/// [`pointed`]'s shape one row down, and for the same reason:
/// `Operation::SetTransition` *"changes nothing you can see and writes nothing
/// to the stream"* — `written` answers `Silent(Surface)` for it and no record
/// in `karakuri-store` carries a quantum, a length or a wipe shape — so there
/// is nothing on the deck for [`apply`] to move and the surface that emits it
/// is what performs it. `View::set_transition` is the only door into that
/// setting, which is where the refusal below lives.
///
/// **What it changes is what the next wipe means**, and that is why the line
/// says the whole row rather than the field that moved: an operator reading
/// *the shape is an iris* still has to know what grid it starts on.
///
/// **A setting no pill can draw is refused**, and it is said rather than
/// swallowed for [`pointed`]'s reason — a press that does nothing and a press
/// that is not bound are the same experience. Nothing this window emits can
/// reach it: the pills name a destination out of the console's own cycles. It
/// is a mapped controller or an MCP call that could, the day either reaches
/// this row.
pub(crate) fn scheduled(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SetTransition { setting } = operation else {
        return None;
    };
    if !view.set_transition(*setting) {
        let at = view.transition();
        return Some(format!(
            "  transition: {setting:?} refused or already there — the row is on `{}`, `{}`, \
             `{}`, and a pill draws only what its own cycle names",
            at.shape_word(),
            at.quantum_word(),
            at.length_word()
        ));
    }
    let at = view.transition();
    Some(format!(
        "  transition: {setting:?} -> no record, and that is settled: it is a surface's own \
         setting. The next fade, crossfade or wipe is `{}` on the `{}`, over {} beat{}",
        at.shape_word(),
        at.quantum_word(),
        at.length,
        match at.length == 1.0 {
            true => "",
            false => "s",
        }
    ))
}

/// **A candidate kept, applied to the lane**, and what to say about it.
/// `None` for every operation that is not it.
///
/// [`scheduled`]'s shape one bay over, and for its reason: `written` answers
/// `Silent(Silent::Surface)` for `Operation::KeepCandidate`, so there is no
/// record for [`apply`] to move a deck with and the surface that draws the row
/// is what performs the press. What it changes is one line in one list.
///
/// **Nothing else moves, and that is the operation rather than a shortfall.**
/// The version is where it was, the store holds every version it held, and the
/// picture is the picture. What a keep says is that a person has looked at
/// this node and is done with it — `console.html`'s *Accepting settles the
/// node and changes nothing on screen*.
///
/// **A row that is not there is said rather than swallowed**, which is
/// [`pointed`]'s rule: nothing this window emits can reach it — the control is
/// the row and a row that is not drawn takes no press — so a line here is a
/// mapped controller or an MCP call arriving at a node with no candidate on
/// it, the day either reaches this row.
pub(crate) fn kept(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::KeepCandidate { deck, node } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let addr = node_addr(ir_layer(node.layer), node.index);
    let before = view.staging.len();
    view.staging
        .retain(|row| row.deck != slot || row.at != Some(*node));
    let letter = deck_letter(*deck);
    if view.staging.len() == before {
        return Some(format!(
            "  keep: deck {letter} {addr} has no candidate row — nothing was outstanding on \
             that node, and the lane is as it was"
        ));
    }
    Some(format!(
        "  keep: deck {letter} {addr} -> KeepCandidate -> no record, and that is settled: the \
         material already changed and its `procedure` record was written at the swap. The row \
         leaves the lane and nothing else moves; {} still waiting",
        match view.staging.len() {
            0 => "nothing".to_owned(),
            n => format!(
                "{n} row{}",
                match n {
                    1 => "",
                    _ => "s",
                }
            ),
        }
    ))
}

/// **Which salt a slot's material is seeded from** — the one it was built at,
/// and the one a load restates when the Set file recorded none.
///
/// Deck A's is [`SEED_SALT`] and every slot after it is one further along, so
/// this deck's four slots are four different simulations of the one procedure
/// [`Sources`] names. **That generalises the reason the second salt was
/// written for and then retires the constant.** `WARM_SEED_SALT` existed so
/// that *the slot the budget parks is a different simulation rather than a
/// second copy of the same one* — an argument about the parked slot, made when
/// the parked slot was the only other slot there was. What it was really
/// saying is that a deck of one picture repeated is not a mixer, and that is
/// true of every slot rather than of deck B, so it is said once here and no
/// constant states a reason that has gone
/// ([`docs/contributing.md` §4](../../../docs/contributing.md)).
///
/// **`+ slot` rather than a table**, because a table of four numbers is four
/// values with nothing to say about each other, and what is wanted is exactly
/// *distinct, and deck A's is the one the CLI's tests use*. Distinctness is
/// then arithmetic rather than four typed numbers nobody re-reads — which
/// `every_slot_is_its_own_simulation` asserts salt by salt, off the Sets the
/// deck actually built rather than off this function.
///
/// **The salts the run was built with, and no others**: a Set loaded into a
/// slot is new *material* and not a new simulation, so a rebuild that derived
/// its own seed would repaint every element in the slot for a reason nobody
/// asked for — which is `Watch::salts`' own argument, met from the loading
/// side.
pub(crate) fn slot_salt(slot: usize) -> u32 {
    SEED_SALT + slot as u32
}

/// **What the derived material a procedure load leaves is a derivation *of***:
/// the Set the slot is filed under, or the pair the run was launched with where
/// it is filed under none.
///
/// **`watch::Aim::set` first**, because that is the one field a load moves and a
/// procedure load does not (ADR-0304, ADR-0338): a slot that has been loaded is
/// running that Set with one layer over it, and the strip has to say so. A slot
/// nobody has loaded is running the launch pair, which no id names — that is
/// the state `Aim::set` is `None` in, and the launch pair is what the strip has
/// been reading since the first frame.
pub(crate) fn base_material(set: Option<&str>, launch: &str) -> String {
    set.map(str::to_owned).unwrap_or_else(|| launch.to_owned())
}

/// **What the strip reads once a layer has been written over what a deck is
/// playing**: `<base> + <kir>`, which is the maintainer's own
/// `drift_night + orbit_wide`.
///
/// So what is on air says what it is made of and never claims to be a Set the
/// library holds — `keep` is what gives it a name (ADR-0338).
pub(crate) fn derived_material(base: &str, procedure: &str) -> String {
    format!("{base} + {procedure}")
}

/// **Write one procedure over the layer it declares and re-aim the slot**, or
/// say why it did not.
///
/// # Where the file comes from, and it is the two tiers and nothing else
///
/// `<store>/procedures/<name>.kir` first and the presets root's
/// `<name>.kir` after it, which is the order the Library bay lists them in and
/// the only two places a procedure row can have come from (ADR-0227's two tiers,
/// ADR-0338's decision 1). The content-addressed artifacts at the store root are
/// **not** searched: that population is the edit history's, addressed by hash,
/// and a name is not one.
///
/// # Which position it lands on, and the limit is recorded rather than designed around
///
/// **The first node of that kind.** A procedure declares one `kind` and nothing
/// about where it goes, and a library row cannot say an index — so the payload
/// carries none, and `L4:0` is the renderer a `kind L4` replaces. The second
/// renderer of a three-renderer Set is unreachable from this row, and the day
/// the Inspector's node head grows a *replace this node* control is the day the
/// payload gains a `NodeAddress` (ADR-0338, stated at the point it bites).
///
/// **Where the slot has no node of that kind the procedure is added as node 0 of
/// it**, which is the case the request is about: a Set of a geometry and a
/// renderer declares no camera, so it holds the built-in orbit at `L3:0` and a
/// `kind L3` row takes that position — the picture changes camera with nothing
/// else moving.
///
/// # What each file already on the slot is
///
/// Read off the files themselves with `history::declared_kind`, which is the one
/// scanner for a `kind` line, and with `compile`'s own fallback where a file
/// declares none — the first node is an L1 and the rest are L4s, which is what
/// a bare pair is. That is one small read per node, on the press, and it
/// compiles nothing (P-0091).
///
/// # The node name is kept, and that is what keeps the edges
///
/// A replaced position keeps the **name the Set gave that node**, because an
/// `edge` and a `bind` in the aim resolve against it: a rebuild that renamed the
/// node would break the wiring the slot is running. A node that is *added* is
/// named after the row, and a name the slot already holds is refused rather than
/// shadowed.
pub(crate) fn overlaying(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    slot: usize,
    aim: &mut Aiming,
    name: &str,
) -> Result<String, String> {
    let (source, tier) = kept_source(root, presets, name)?;
    let kind = karakuri_environment::history::declared_kind(&source).ok_or_else(|| {
        format!(
            "`{name}` declares no `kind`, so there is no layer to write it over — a procedure \
             says which layer it implements in a `kind` line, and this one says nothing"
        )
    })?;
    // **Head and rest are one list here**, because *the first node of that kind*
    // is a question about the slot's files in order and the split is only how a
    // `watch::Aim` carries them.
    let mut files: Vec<karakuri_environment::compile::Named> = std::iter::once(aim.at.head.clone())
        .chain(aim.at.rest.iter().cloned())
        .collect();
    let at = files.iter().enumerate().find_map(|(index, file)| {
        let source = std::fs::read(&file.path).ok()?;
        let declared = karakuri_environment::history::declared_kind(&source)
            .unwrap_or(if index == 0 { "L1" } else { "L4" });
        (declared == kind).then_some(index)
    });
    let (at, added) = match at {
        Some(at) => (at, false),
        None => (files.len(), true),
    };
    if added && files.iter().any(|file| file.name.as_deref() == Some(name)) {
        return Err(format!(
            "deck {} already holds a node called `{name}`, and this row would add a second — \
             rename one of them and load again",
            deck_letter(slot as u8)
        ));
    }
    let path = karakuri_environment::scratch::place(
        root,
        &karakuri_environment::scratch::node_name(slot, at, name),
        &String::from_utf8_lossy(&source),
    )?;
    match added {
        // **Named after the row**, because nothing in the slot named it: a node
        // added here is addressed by the name an operator can read off the row
        // they pressed.
        true => files.push(karakuri_environment::compile::Named {
            name: Some(name.to_string()),
            path,
        }),
        // **The node keeps the name the Set gave it**, which is what an `edge`
        // resolves against — see this function's own head.
        false => files[at].path = path,
    }
    let mut files = files.into_iter();
    let head = files
        .next()
        .ok_or_else(|| String::from("this deck is running no files at all"))?;
    let rest: Vec<karakuri_environment::compile::Named> = files.collect();
    // **Every other field restated**, which is `Aiming::changed`'s single
    // derivation: the layering, the fold, the capacity, the seed and the salts,
    // the camera, the overrides, the wiring, the grants and the Set this slot is
    // filed under all come back as the slot's own rather than as a default
    // (ADR-0314). `Aim::set` is among them, which is why the history goes on
    // filing under the base.
    aim.changed(|at| {
        at.head = head;
        at.rest = rest;
    })
    .map_err(|()| {
        format!(
            "deck {}'s build worker is gone, so `{name}` cannot be built; what is on that deck \
             keeps running",
            deck_letter(slot as u8)
        )
    })?;
    let where_ = match added {
        true => format!("added as {kind}:0, which this deck had no node for"),
        false => format!("written over {kind}:0"),
    };
    Ok(format!(
        "  load: `{name}` ({tier}) {where_} on deck {} — the slot is recompiling on the worker \
         with every other layer where it was, and the staging lane says whether the build \
         landed, was overloaded or did not compile",
        deck_letter(slot as u8)
    ))
}

/// **A procedure's bytes, out of whichever tier holds it**, with the word for
/// the tier so the sentence a press prints says where the file came from.
///
/// The operator's own first and what ships after it, which is the order the
/// listing draws them under `all` and `presets`: a name kept in this store is
/// this store's answer, and nothing this program does writes where the presets
/// are (P-0096).
pub(crate) fn kept_source(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    name: &str,
) -> Result<(Vec<u8>, &'static str), String> {
    if let Ok(store) = Store::open(root) {
        if let Ok(source) = store.read_procedure(name) {
            return Ok((source, "kept"));
        }
    }
    let shipped = presets.map(|dir| dir.join(format!("{name}.kir")));
    if let Some(path) = shipped {
        if let Ok(source) = std::fs::read(&path) {
            return Ok((source, "shipped"));
        }
    }
    Err(format!(
        "no procedure named `{name}` — this store's `{}/` does not hold one and the presets root \
         does not ship one, so the row it was listed under has gone since the listing was built",
        Store::PROCEDURES
    ))
}

/// **A deck's renderers folded or overdrawn, performed** — the Inspector deck
/// head's fold pressed, and `None` for every operation that is not one.
///
/// # It is [`played`]'s shape with one field instead of every field
///
/// A library load re-points a slot at a different Set's files; this re-points a
/// slot at *the files it is already on*, with the layering changed. Both are
/// one `Aiming::changed`, both are judged by the same watchdog, and neither
/// touches the deck — see [`Aiming::changed`], where the argument is, and
/// `docs/adr/0314-…`, which is the record.
///
/// **`written` answers `Silent(NoRecord)`**, exactly as it does for
/// `Operation::LoadSet`, so the surface that names it is the surface that
/// performs it and there is nothing for [`apply`] to do. What a session stream
/// has for a layering is `Record::Merge`, which is a **Set file's** statement
/// about a Set and carries no slot; nothing in the vocabulary says *the Set in
/// slot 3 composites*, and inventing a record here would be inventing the
/// record stream (ADR-0046's rule, met from the panel).
///
/// # A press that asks for the state the slot is in is refused rather than sent
///
/// Not because asking twice is wrong — [`DeckHead::compositing`] names a
/// destination, and naming the one you are on is how the anchor beside it
/// re-anchors — but because *here* it would buy a recompile of the whole slot
/// and change nothing about the picture, which is a cost paid for nothing
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
/// The chip cannot produce one, since it reads the state the frame drew; MIDI,
/// a key or a model can, the day any of them names this operation.
///
/// Every failure is a sentence and none of them moves anything: a slot the deck
/// has not got, or a build worker that has gone.
///
/// **A free function over the aims and not over [`Gfx`]**, which is [`rewired`]'s
/// arrangement and its reason: the whole of what this decides is the field, the
/// refusal and the sentence, and none of the three needs a window, a device or
/// a `Deck` to check. [`played`] beside it takes the program because a load
/// reads a store and writes the strip's name; this one touches nothing but the
/// aim.
pub(crate) fn composited(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetCompositing { deck, compositing } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  composite: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    let want = match compositing {
        true => karakuri_engine::set::Layering::Composite,
        false => karakuri_engine::set::Layering::Overdraw,
    };
    let word = match compositing {
        true => "composite",
        false => "overdraw",
    };
    if aim.at.layering == want {
        return Some(format!(
            "  composite: deck {letter} is already set to {word} its renderers — nothing was \
             sent, because a re-aim rebuilds the whole slot and this one would land on the same \
             picture"
        ));
    }
    match aim.changed(|at| at.layering = want) {
        Ok(()) => Some(format!(
            "  composite: deck {letter} re-aimed to {word} its renderers — the slot is \
             recompiling on the worker, and the staging lane says whether the build landed, was \
             overloaded or did not compile"
        )),
        Err(()) => Some(format!(
            "  composite: deck {letter} will {word} its renderers from the next build on, but \
             this slot's build worker has ended, so nothing will rebuild and what is on that \
             deck is still running"
        )),
    }
}

/// **A deck's element count moved, performed** — the Inspector deck head's
/// capacity chip pressed, and `None` for every operation that is not one.
///
/// # It is [`composited`]'s shape with a different field of the aim
///
/// A capacity is one field of the description a slot's watcher is pointed at,
/// so this restates the other thirteen and sends it and the worker recompiles
/// the slot off the render thread — the route ADR-0228 opened and ADR-0314
/// walked, and the reason a *setter* on `Set` was never what this waited on.
/// `written` answers `Silent(NoRecord)` for `SetProperty` exactly as it does
/// for `SetCompositing` and `LoadSet`, so there is nothing for [`apply`] to do:
/// `Record::Capacity` is a **Set file's** statement about a Set and carries no
/// slot. A session replayed therefore does not come back at a capacity a hand
/// stepped to — the load's cost, unchanged in size; **a deck kept does**, since
/// a keep writes one `capacity` record per geometry off what the Set is running
/// at (`docs/adr/0328-…`).
///
/// # It is the whole slot, and that is the aim's shape rather than a shortcut
///
/// `watch::Aim::capacity` is one `Option<u32>` and is `--capacity`'s own field:
/// *"`--capacity` overrides every source"*. So a Set holding two geometries
/// runs both at this number. That is ADR-0228's recorded limit met from the
/// asking side rather than worked around, and it is why
/// `karakuri_operation::Property::Capacity` names no node.
///
/// **A press asking for the capacity the slot is already aimed at is refused
/// with a sentence** and nothing is sent, on the fold's terms: it would buy a
/// recompile of the whole slot and land on the same picture. The chip cannot
/// produce one — its step is strictly above what the slot is running — and a
/// **model can**, since `set_property` names the number outright and arrives
/// here as an `Acted::Emitted` like any press. That is why the guard is here
/// and not in the console: what may be asked for is not a surface's to decide,
/// so every way in meets the same wall in the same sentence
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// **A free function over the aims**, for [`composited`]'s reason: the field,
/// the refusal and the sentence are the whole of what it decides.
pub(crate) fn resized(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetProperty {
        deck,
        property: karakuri_operation::Property::Capacity { elements },
    } = operation
    else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  capacity: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    if aim.at.capacity == Some(*elements) {
        return Some(format!(
            "  capacity: deck {letter} is already aimed at {elements} elements a geometry — \
             nothing was sent, because a re-aim rebuilds the whole slot and this one would land \
             on the same picture"
        ));
    }
    match aim.changed(|at| at.capacity = Some(*elements)) {
        Ok(()) => Some(format!(
            "  capacity: deck {letter} re-aimed to {elements} elements a geometry — the slot is \
             recompiling on the worker, and the staging lane says whether the build landed, was \
             overloaded or did not compile. A number outside what a geometry declares is refused \
             there, by name and with the range"
        )),
        Err(()) => Some(format!(
            "  capacity: deck {letter} will run at {elements} elements a geometry from the next \
             build on, but this slot's build worker has ended, so nothing will rebuild and what \
             is on that deck is still running"
        )),
    }
}

/// **An input rewired, performed** — a pick out of a `uses` line's card, and
/// `None` for every operation that is not one.
///
/// # It is the route a model's `wire_input` already takes, reached from a press
///
/// [`rewired`] is the whole of what a rewiring decides — the run's wiring, the
/// re-aim and the sentence — and it was written for the MCP surface, one
/// request per frame, with a slot number it does not trust. A press is one
/// request of exactly that shape, so this hands it one and prints what comes
/// back: the panel and a model rewire through one function, and a defect in
/// either is a defect in both rather than in whichever was tried
/// ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
///
/// **Nothing is validated here.** The card offers nodes the pane could see and
/// a name the Set cannot use is refused where the Set is *built*, by name and
/// with what the Set does hold — which is the wall every way in meets, in one
/// sentence
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// **`written` answers `Silent(NoRecord)`** for `WireInput` and still does:
/// `Record::Edge` is a **Set file's** statement about a Set and carries no
/// slot, so a session replayed does not come back rewired where a hand asked
/// for it — and **a keep does**, since a keep writes the run's edges into the
/// file it saves. That is `SetProperty`'s division and `LoadSet`'s hole, not a
/// new one (`docs/adr/0329-…`).
pub(crate) fn wired_input(
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Aiming],
    slot_count: usize,
    operation: &Operation,
) -> Option<String> {
    let Operation::WireInput {
        deck,
        node,
        slot,
        to,
    } = operation
    else {
        return None;
    };
    let asked = [(
        usize::from(*deck),
        karakuri_engine::set::Edge {
            node: node.clone(),
            slot: slot.as_str().into(),
            to: to.clone(),
        },
    )];
    // **One request, so one answer** — `rewired` answers per request and this
    // hands it exactly one, which is why the `into_iter().next()` below cannot
    // be an empty list.
    let said = rewired(&asked, edges, aims, slot_count)
        .into_iter()
        .next()?;
    Some(match said {
        Ok(line) => format!("  wire: {line}"),
        Err(line) => format!("  wire: {line}"),
    })
}

/// **A deck's published interface narrowed or widened, performed** — a
/// parameter row's publish mark pressed, and `None` for every operation that is
/// not one.
///
/// # It is a field of the aim, which is what makes the choice survive
///
/// `watch::Aim::published` is the interface a slot's watcher states at every
/// build, and it has been empty in every run this program has ever had — an
/// empty list *is* **publish everything**, so nothing had to fill it until
/// something narrowed. This fills it, and the reason it is the aim rather than
/// a writer into the live `Set` is the one thing that decides between them: a
/// live write is wiped by the next rebuild, and the next rebuild is the
/// operator's own next save of any `.kir` in the deck. A control that undoes
/// itself on an unrelated act is the defect ADR-0280 §6 named for parameters
/// and ADR-0282 closed; there is no `Set::carry_moved_from` for an interface,
/// so the aim is where it has to live (`docs/adr/0329-…`).
///
/// **The cost is a recompile for a choice about a display**, and it is named
/// rather than hidden: the build is the same files at the same capacity, so it
/// is a build that has already landed once, and the Staging lane carries the
/// verdict like every other.
///
/// **`written` answers `Silent(NoRecord)`**, and here that is a **gap in the
/// format** rather than a record with no slot: nothing in this vocabulary says
/// what a Set publishes, in a Set file or in a session. So a replay does not
/// come back narrowed and **neither does a keep** — which is what makes this
/// the weakest of the four re-aims on that row, and both manual pages say so.
///
/// **An empty list is not nothing.** `Publish { controls: [] }` asks for *every
/// declared control published*, which is what an unnarrowed deck is, and it is
/// what a press that takes the last control off the interface would mean if
/// anything could produce one — nothing can, because taking a row off leaves
/// the rest on it.
pub(crate) fn attended(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::Publish { deck, controls } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  publish: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    let published: Vec<karakuri_engine::set::Published> = controls
        .iter()
        .map(|control| karakuri_engine::set::Published {
            name: control.name.clone(),
            at: control.node.map(|node| (ir_layer(node.layer), node.index)),
            key: control.key.clone(),
            range: control.range,
        })
        .collect();
    let shown = published.len();
    match aim.changed(|at| at.published = published) {
        Ok(()) => Some(format!(
            "  publish: deck {letter} re-aimed to publish {shown} control{} — the slot is              recompiling on the worker, and the staging lane says whether the build landed, was              overloaded or did not compile. What is off the interface is still written by              `--param`, by a `param` record and by a model naming its address",
            match shown {
                1 => "",
                _ => "s",
            }
        )),
        Err(()) => Some(format!(
            "  publish: deck {letter} will publish {shown} control{} from the next build on, but              this slot's build worker has ended, so nothing will rebuild and what is on that              deck is still running",
            match shown {
                1 => "",
                _ => "s",
            }
        )),
    }
}

/// **A deck re-seeded, performed** — the Inspector deck head's `re-salt`
/// capsule pressed, and `None` for every operation that is not one.
///
/// # The salts are cleared and the seed is stated, which is one derivation
///
/// `watch::Aim` carries both a `seed_salt` for the slot and a `salts` list per
/// geometry, and the list wins where it is filled: `Set::build` reads a
/// recorded salt and falls back to `derived_salt(seed_salt, ordinal)`. So a
/// re-salt that wrote only the seed would move nothing on a slot filled from a
/// Set file, which records one `seed` line per geometry. It **clears the list**
/// instead of rewriting it, which is the same numbers with the arithmetic left
/// where it belongs: the engine derives each geometry's salt from the slot's,
/// and this program does not keep a second copy of that function
/// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
///
/// # Nothing here invents the number
///
/// The salt arrives in the operation, because the console was handed it: the
/// pane reads `Set::source_salts`, which is what the slot is actually running,
/// and the next value of the sequence comes off
/// `karakuri_engine::set::derived_salt` — so a press names a destination like
/// every other control on this row, the same press from the same place lands on
/// the same picture twice, and nothing on this panel produces a frame a later
/// run cannot produce again
/// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
///
/// **Nothing is refused.** The sequence goes forward, so a press cannot ask for
/// the salt the slot is already on, and a re-seed always changes the picture —
/// which is what the fold's *already in that state* guard exists for and this
/// one does not need.
pub(crate) fn re_salted(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetProperty {
        deck,
        property: karakuri_operation::Property::Seed { salt },
    } = operation
    else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  re-salt: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    match aim.changed(|at| {
        at.seed_salt = *salt;
        at.salts.clear();
    }) {
        Ok(()) => Some(format!(
            "  re-salt: deck {letter} re-aimed to seed {salt} — the slot is recompiling on the \
             worker, and its randomness moves while its structure does not. The staging lane says \
             whether the build landed, was overloaded or did not compile"
        )),
        Err(()) => Some(format!(
            "  re-salt: deck {letter} will be seeded from {salt} from the next build on, but this \
             slot's build worker has ended, so nothing will rebuild and what is on that deck is \
             still running"
        )),
    }
}

/// **What a [`karakuri_operation::Revision`] asked for, as a refusal names
/// it** — a version by the name it was filed under, or a node by its address.
///
/// One function because the two refusals about the *deck* are the same
/// sentence whichever arm arrived: a slot the deck has not got and a deck
/// playing no Set are answered before anything is read, and what they have to
/// name is only what was asked for.
pub(crate) fn asked_for(revision: &karakuri_operation::Revision) -> String {
    match revision {
        karakuri_operation::Revision::Picked(name) => format!("`{name}`"),
        karakuri_operation::Revision::Previous(node) => format!(
            "the version before {}'s",
            node_addr(ir_layer(node.layer), node.index)
        ),
    }
}

/// **The half of [`restored`] that reaches a disk**, split out for the reason
/// [`seeded`] is a free function: `main` cannot be entered from a test, a
/// `Gfx` cannot be built without a device, and what this does is worth
/// asserting — it writes over the file a deck is playing from.
///
/// Everything it needs is an argument: the store to walk, the slot and its
/// letter, the Set the slot is running, the revision that was asked for, and
/// where that slot's nodes are ([`karakuri_mcp::Slots`], the
/// run's one published layout, which the caller reads off [`Engine::pointing`]).
/// The refusals here are the ones that are about **files** — a version that is
/// not in the listing, a node with nothing behind the one it is playing, a
/// node this slot does not hold, and a file that will not be read or written —
/// where the two about the *deck* are the caller's and are answered before
/// this is reached.
///
/// # One function and two ways of naming the file
///
/// `karakuri_operation::Revision` has two arms because two surfaces can ask
/// and each says the half it holds (ADR-0308), and what differs between them
/// is **which row of this listing** — nothing after that. So the walk, the
/// read, the address and the write are one path, and the arms are one `match`
/// over the same `Vec<Version>`:
///
/// - `Picked` is a name the Library bay's `history` scope handed over, matched
///   back against the listing that produced it by rebuilding each row's
///   spelling — `SetTransfer::Take`'s own arrangement, and the reason no
///   surface here spells a path.
/// - `Previous` is a node the Staging lane's row handed over, and the version
///   is **the one before the one running**: the listing is most recent first,
///   the newest entry for that node is what the slot is playing — the history
///   is gated on compiling and not on landing, so a version that was stopped
///   for cost is filed too — and the entry after it is the step back
///   (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
///   A node with exactly one version in the listing is refused naming what is
///   in the way, which is `P-0083`: it says the node has nothing behind what
///   it is playing rather than that the press failed.
///
/// **The narrowing to the Set is both arms'**, and it is the same narrowing
/// for the same reason — a version filed under no Set is a version of nothing.
pub(crate) fn put_back(
    store: &std::path::Path,
    slot: usize,
    letter: &str,
    id: &str,
    revision: &karakuri_operation::Revision,
    slots: &karakuri_mcp::Slots,
) -> String {
    let asked = asked_for(revision);
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        Err(why) => {
            return format!(
                "  put back: the edit history could not be read: {why} — nothing moved, and \
                 what is on deck {letter} is still running"
            );
        }
    };
    // **`Some(id)` and never `None`**, which is [`walked`]'s filter said again
    // where it decides what is written rather than what is drawn: a version
    // filed under no Set is a version of nothing, and landing one because a
    // `None` read as a wildcard would put another run's edit on a deck.
    let of_the_set = || {
        found
            .versions
            .iter()
            .filter(|version| version.set.as_deref() == Some(id))
    };
    let version = match revision {
        karakuri_operation::Revision::Picked(picked) => {
            let Some(version) = of_the_set().find(|version| version_row(version) == *picked) else {
                return format!(
                    "  put back: `{picked}` is not one of `{id}`'s versions in the last \
                     {HISTORY_MOST} this store wrote — a day directory an operator deleted by \
                     hand is the ordinary way that happens, since `rm -rf history/YYYY/MM` is \
                     the whole of the retention policy; nothing moved, and deck {letter} is \
                     still playing `{id}`"
                );
            };
            version
        }
        karakuri_operation::Revision::Previous(node) => {
            let layer = setfile::kind_name(ir_layer(node.layer));
            let addr = node_addr(ir_layer(node.layer), node.index);
            // **Most recent first is `history::list`'s own order**, kept
            // rather than re-sorted here: the newest is what the slot is
            // running and the one after it is the step back.
            let mut chain = of_the_set()
                .filter(|version| version.layer == layer && version.index == node.index as usize);
            let running = chain.next();
            let Some(version) = running.and(chain.next()) else {
                return format!(
                    "  put back: deck {letter} {addr} has {} in `{id}`'s history, so there is \
                     nothing before what it is playing to step back to — the history keeps \
                     every version that compiled, and this node has only ever had the one. \
                     Nothing moved.",
                    match running {
                        Some(_) => "one version",
                        None => "no version",
                    }
                );
            };
            version
        }
    };
    let source = match std::fs::read(&version.file) {
        Ok(source) => source,
        Err(e) => {
            return format!(
                "  put back: {}: {e} — the row is a name and the file behind it is gone, so \
                 nothing moved and deck {letter} is still playing `{id}`",
                version.file.display()
            );
        }
    };
    let target = match slots.file(slot, version.layer, version.index) {
        Ok(path) => path.clone(),
        Err(why) => {
            return format!(
                "  put back: deck {letter} does not hold the node {asked} was a version \
                 of — {why}; `{id}` has been edited since, or this version came off another \
                 slot. Nothing moved."
            );
        }
    };
    match std::fs::write(&target, &source) {
        Ok(()) => format!(
            "  put back: deck {letter} {}:{} <- `{}` -> written into {}; the worker \
             builds it and the budget judges it, and the version it replaces is kept because \
             the rebuild compiles it",
            version.layer,
            version.index,
            version_row(version),
            target.display()
        ),
        Err(e) => format!(
            "  put back: {}: {e} — nothing moved, and what is on deck {letter} is still \
             running",
            target.display()
        ),
    }
}

/// **The record, applied to the deck**, and what to say about it.
///
/// **This is not the half ADR-0185 promised to delete, and it did not go with
/// it.** Turning an `Operation` into a `Record` was the shortcut — that
/// function is gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and **what the deck says
/// afterwards** — which is where the next frame's strip comes from.
///
/// # It takes the look as well as the deck, and that is not a second target
///
/// `Record::Look` is the one record here that does not name a slot: the look
/// is what *every* sink is drawn under, so it is `Engine::look` rather than
/// anything on the deck ([`Engine::look`], and `karakuri_engine::frame::Look`
/// for why the master out is deliberately not in it). Handing both in is what
/// keeps this one function the only place a record becomes a movement — a
/// second `apply_look` beside it would be the second route into the engine
/// that P-0090 exists to refuse.
pub(crate) fn apply(
    record: &Record,
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
) -> Option<String> {
    // **A slot the deck has not got is refused rather than indexed**, and the
    // guard is [`held`] rather than a closure here, because the key arms in
    // `window_event` need the same answer one step earlier: a press reads the
    // trim it is stepping from before it can name where it is going, so a
    // guard on the record alone would be a read that panicked on its way to a
    // refusal. The argument for refusing at all is at that function.
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.opacity(slot)
            ))
        }
        // **The mode comes back off the wire name, and an unknown one is
        // refused rather than defaulted.** `Record::Blend` carries a `String`
        // because what a mode is allowed to be is the engine's to say, so this
        // is the engine saying it — `mix::change` refuses the same way, with
        // the sentence `karakuri-cli`'s `no_such_blend` writes. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `BlendMode::ALL`, so this is the guard rather
        // than the message.
        Record::Blend { slot, ref mode } => {
            let slot = held(deck, slot.0)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot.0),
                deck.blend(slot).name()
            ))
        }
        // **The one record here that is followed by a governor pass**, and it
        // is not a flourish: `Deck::set_residency` writes the request *and
        // grants it*, so a harness that stopped there would put a slot the
        // budget has no room for into `Priming` and draw a primed deck the
        // governor never admitted. That is
        // [ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)
        // exactly — the panel's parked deck is the governor's verdict or it is
        // a drawing of one — and it is `karakuri-cli`'s own order, where
        // `mix::Change::Residency` sets the level and calls `govern` beside
        // it. The report is dropped here rather than printed: the line below
        // says what the deck ended up at, which is the half this window shows.
        Record::Residency { slot, ref level } => {
            let slot = held(deck, slot.0)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot.0),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        // **The whole mask, because the record is a state and not an ask.**
        // `Record::Mask` carries a shape, an angle, a position and a softness,
        // and `Deck::set_mask` is what it decodes to — the engine says so at
        // that setter. Reaching for `Deck::set_mask_shape` instead, to keep a
        // running wipe alive, would be this file decoding a record by picking
        // two fields out of it and dropping the softness on the floor: a
        // second route to the deck, where P-0090 is that every control ends in
        // the same record. **So a shape press stops a wipe on that deck**, and
        // that is not a fault here — it is the honest limit
        // `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`
        // states about the record stream, met by the first surface to make the
        // press
        // ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
        //
        // **The shape comes back off the wire name**, refused rather than
        // defaulted, exactly as the blend's and the residency's do. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `WipeKind`'s three and the record is written
        // from `WipeKind::name`.
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(deck, slot.0)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind}, \
                 angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} \
                 at {:.3} rad, front at {:.3}",
                deck_letter(slot.0),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        // **The whole look, because the record is a state and not an ask.**
        // `Record::Look` carries the operator, the exposure and the white
        // point together for its own stated reason — a stream that set a level
        // without naming the operator would describe a look nobody can
        // reconstruct — and each of the two controls asks for one of the three
        // (ADR-0192). The other two arrive here already filled in from the
        // reading [`reading`] took, so this writes what it is given and picks
        // nothing out of it, exactly as the mask arm does.
        //
        // **The operator comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's and the shape's do.
        // Nothing in this file can produce a name the engine has not got: the
        // capsule only ever emits one of `Tonemap`'s four and the record is
        // written from `Tonemap::name`, which is the same lower-case spelling
        // `mix::op_wire_name` parses.
        //
        // **No `set_tonemap` call.** `compose` uploads the tone-map uniform
        // every frame from the `Committed` the closure hands back, so writing
        // the field *is* the write — and a `Present::set_tonemap` here would be
        // a second writer, with the last one each frame winning.
        Record::Look {
            ref op,
            exposure,
            white_point,
        } => {
            let op = mix::parse_op(op)?;
            *look = Look {
                op,
                exposure,
                white_point,
            };
            Some(format!(
                "  look: -> Record::Look {{ op: {op_name}, exposure: {exposure:.3}, \
                 white_point: {white_point:.3} }} -> every sink is drawn under {op_name} at \
                 exposure {exposure:.3}",
                op_name = mix::op_wire_name(op)
            ))
        }
        // **A scheduled move, and the first record this window applies that
        // does not land now.** `Record::Transition` carries the slot, which
        // control is moving, where it ends up, the beat it starts on, how long
        // it lasts and the shape it eases with — and the one thing it does not
        // carry is where the move starts *from*, because that is where the
        // control already is at the moment the record is applied. Reading it
        // here rather than off the record is what makes a replay fade from
        // where the run did, and it is `karakuri-cli`'s `schedule_from`, in
        // the one place this window needs it.
        //
        // **It landed with the `go` capsule, and until then the window drew a
        // wipe's other records and dropped this one on the floor**: the
        // arriving deck was masked to nothing and put on air, and the front
        // never travelled. A record with no arm here is silent — the `_` at
        // the foot of this match — which is why the gap was invisible.
        //
        // **The two names come back off the wire, refused rather than
        // defaulted**, as the blend's, the residency's and the sync mode's do:
        // a control this engine has not got and a curve it cannot ease with
        // are both a record from a stream this build does not understand, and
        // guessing at either would schedule a move nobody wrote.
        Record::Transition {
            slot,
            ref control,
            to,
            start,
            beats,
            ref curve,
        } => {
            let slot = held(deck, slot.0)?;
            let control = Control::from_name(control)?;
            let curve = karakuri_engine::binding::Curve::parse(curve)?;
            let from = match control {
                Control::Gain => deck.gain(slot),
                Control::Opacity => deck.opacity(slot),
                Control::MaskPosition => deck.mask(slot).position(),
            };
            deck.schedule(karakuri_engine::Transition::new(
                slot.index(),
                control,
                from,
                to,
                start,
                beats,
                curve,
            ));
            Some(format!(
                "  transition: deck {} {} {from:.3} -> {to:.3} -> Record::Transition {{ \
                 start: {start:.3}, beats: {beats:.3}, curve: {} }} -> \
                 deck.transitions_on({slot}) = {}",
                deck_letter(slot.0),
                control.name(),
                curve.name(),
                deck.transitions_on(slot).count()
            ))
        }
        // **One level on the whole fold, and the one arm here that names no
        // slot at all.** `Record::MasterOut` carries a number and nothing
        // else, so unlike the look and the mask there is no other half of it
        // to fill in from what is running — which is why the reading below
        // has no arm for this control (ADR-0224).
        //
        // **The engine clamps and this does not.** `Deck::set_out` floors at
        // zero and is deliberately open above 1.0, through the same
        // `clamp_gain` the per-slot gain goes through, because the mix is HDR
        // and this level is applied to values a tone mapper has not seen —
        // [P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md).
        // A clamp here would be a second opinion about a range the setter
        // already holds, which is the rule the whole conversion is written
        // under.
        //
        // **No `cancel` to worry about**, where the gain and the fader each
        // stop whatever was moving them: a `Control` is per slot and this is
        // not, so nothing in the engine can be moving it and there is nothing
        // for a hand to win against.
        // **The one record that reaches inside a Set**, and the row the
        // Inspector bay was blocked on. `Deck::write_param` is the public road
        // and it compiles nothing — the map is packed into the uniform by the
        // next `Set::prepare`, so the value is on screen on the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the one arm
        // in this function that does not take the short path, and the reason is
        // the expansion: a `vec3` value is three writes under the component
        // keys ADR-0268 made, and spelling that a second time in this file is
        // the drift `karakuri-operation-record` exists to end. It is also what
        // refuses a slot this deck has not got, in the sentence every other
        // surface refuses one with — so `held` is not asked first here.
        Record::Ride { .. } => {
            let writes = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Ride { slot, writes })) => {
                    let mut reached = 0;
                    for write in &writes {
                        match deck.write_param(EngineSlot(slot as u8), write) {
                            Ok(n) => reached += n,
                            Err(refused) => return Some(format!("  {refused}")),
                        }
                    }
                    (slot, writes, reached)
                }
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            let (slot, writes, reached) = writes;
            match reached {
                0 => Some(format!(
                    "  {}",
                    karakuri_environment::no_such_param(slot, &writes[0].key)
                )),
                _ => Some(format!(
                    "  knob: deck {} -> WriteParam -> Record::Ride -> {} on {reached} node(s)",
                    deck_letter(slot as u8),
                    writes
                        .iter()
                        .map(|w| format!("{} = {:.3}", w.key, w.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        // **What is driving a knob, and the taking of it back** — the other
        // record that reaches inside a Set, and the one the Inspector's
        // sensitivity row was blocked on. `Deck::bind` and `Deck::unbind` are
        // the public roads and neither compiles anything: a binding is
        // resolved by the next `Set::prepare`, so an attachment is riding on
        // the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the ride's
        // reason one arm up: the three diagnostics a binding owes — a `bpm`
        // source, an `octaves` without `fbm`, a generator on a binding that is
        // not to `noise` — belong to `setfile::binding_from_record` and are
        // said in its words on every route, so spelling them a second time
        // here is the drift `karakuri-operation-record` exists to end.
        Record::Source { .. } => {
            let (slot, key, bound) = match karakuri_environment::mix::change(
                record,
                deck.slot_count(),
            ) {
                Ok(Some(karakuri_environment::mix::Change::Source {
                    slot,
                    layer,
                    index,
                    key,
                    binding,
                })) => match binding {
                    Some(binding) => {
                        let signal = binding.signal.clone();
                        let curve = binding.curve.name();
                        let range = binding.range;
                        match deck.bind(EngineSlot(slot as u8), binding) {
                                karakuri_engine::set::Bound::Yes => (
                                    slot,
                                    key,
                                    format!(
                                        "{signal} through {curve} onto [{:.2}, {:.2}]",
                                        range[0], range[1]
                                    ),
                                ),
                                karakuri_engine::set::Bound::NoSuchParam => {
                                    return Some(format!(
                                        "  {}",
                                        karakuri_environment::no_such_param(slot, &key)
                                    ))
                                }
                                karakuri_engine::set::Bound::NoSuchControl => {
                                    return Some(format!(
                                        "  slot {slot}: `{signal}` is not a control this Set                                          publishes"
                                    ))
                                }
                            }
                    }
                    None => match deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                        true => (slot, key, "nothing — taken back".to_string()),
                        false => {
                            return Some(format!("  slot {slot}: nothing was driving `{key}`"))
                        }
                    },
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  source: deck {} -> Record::Source -> `{key}` is driven by {bound}",
                deck_letter(slot as u8)
            ))
        }
        // **Who may move one node**, and the writer ADR-0211 said the engine
        // owed. It is not enforcement: nothing writes a parameter on an
        // agent's behalf here, so what a level reaches today is
        // `Set::write_param`'s wildcard refusal — a bare-name control over
        // nodes that no longer agree is refused whole from the next press.
        Record::Authority { .. } => {
            let (slot, level) = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Authority {
                    slot,
                    layer,
                    index,
                    authority,
                })) => match deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                    true => (slot, authority),
                    false => {
                        return Some(format!(
                            "  slot {slot}: no node {}:{index} to speak for",
                            karakuri_environment::setfile::layer_name(
                                karakuri_environment::setfile::layer_of(layer)
                            )
                        ))
                    }
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  authority: deck {} -> SetAuthority -> Record::Authority -> {}",
                deck_letter(slot as u8),
                level.name()
            ))
        }
        Record::MasterOut { value } => {
            deck.set_out(value);
            Some(format!(
                "  master: out -> SetMasterOut {{ out: {value:.3} }} -> Record::MasterOut -> \
                 deck.out() = {:.3}, at the entry to the master chain",
                deck.out()
            ))
        }
        // **The chain itself, one record for all three passes.** Written whole
        // for `Record::Look`'s reason and applied whole: the value lands on
        // [`Engine::chain`] and the frame loop hands it to
        // `Present::set_chain`, so nothing here touches a uniform. That is the
        // look arm's arrangement one pass upstream, and it is why there is no
        // `set_chain` call in this function.
        //
        // **The cut comes back off the wire word, refused rather than
        // defaulted**, as the tone map operator and the blend mode do: a cut
        // this engine has not got is a stream saying something this build
        // cannot draw, and a default would silently play the other picture.
        //
        // **No clamp here either.** `Chain::clamped` is the wall and it is
        // inside the setter, so a stream carrying 4.0 meets the same ceiling
        // a fader does.
        Record::MasterChain(ref want) => {
            let mut slots = Vec::with_capacity(want.slots.len());
            for slot in &want.slots {
                slots.push(karakuri_engine::SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut: match &slot.cut {
                        None => None,
                        Some(word) => Some(Cut::parse(word)?),
                    },
                    params: slot.params.clone(),
                });
            }
            let said = format!(
                "  master: chain -> Record::MasterChain -> {} slot{} — {}",
                slots.len(),
                if slots.len() == 1 { "" } else { "s" },
                if slots.is_empty() {
                    "the frame is the mix".to_string()
                } else {
                    slots
                        .iter()
                        .map(|s| {
                            let cut = s
                                .cut
                                .map(|c| format!(" ({})", c.name()))
                                .unwrap_or_default();
                            format!("{}{cut}", &s.procedure[..s.procedure.len().min(14)])
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            *chain = slots;
            Some(said)
        }
        // **The whole of one slot's clock, because the record is a state and
        // not an ask.** `Record::Transport` carries the sync mode, the anchor
        // and the scrub together for a stated reason — a scrub position
        // without the mode and the anchor beside it *"would replay a slot onto
        // a grid it was never on"* — and `Deck::set_transport` is what it
        // decodes to. The mode and the anchor arrive here unchanged, from the
        // reading [`reading`] took a moment earlier; only the scrub has moved.
        //
        // **The mode comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's, the shape's and the
        // operator's do. Nothing in this file can produce a name the engine
        // has not got: the scrub only ever writes back the mode it just read
        // off the same deck, and the record is written from `Sync::name`.
        //
        // **`set_transport` refuses, and the refusal is dropped here on
        // purpose.** It refuses a mode this slot's material cannot honour, and
        // a scrub cannot present one — it names the mode the slot is already
        // in, which the slot is in because the engine allowed it. What it
        // guards against is the case the engine names at `sync_allowed`: a
        // swap that puts accumulating material into a slot that is beat-synced
        // makes the mode it is *already in* unavailable, *"and whatever wires
        // swapping to this owes it"*. **Both slots are watched now, so that
        // case is reachable**: save an accumulating procedure into a
        // beat-synced slot and the next scrub is refused. What should be
        // printed then is the deck's own refusal rather than this arm's line,
        // and it is what this owes.
        Record::Transport {
            slot,
            ref sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = held(deck, slot.0)?;
            let mode = EngineSync::from_name(sync)?;
            deck.set_transport(slot, mode, anchor_bpm, scrub_beats)
                .ok()?;
            Some(format!(
                "  scrub: deck {} -> ScrubDeck {{ deck: {slot} }} -> Record::Transport {{ \
                 sync: {sync}, anchor_bpm: {anchor_bpm:.1}, scrub_beats: {scrub_beats:+.2} }} \
                 -> deck.transport({slot}).scrub_beats() = {:+.2} beats",
                deck_letter(slot.0),
                deck.transport(slot).scrub_beats()
            ))
        }
        // **A choice and not a position, so nothing interpolates and nothing
        // is left running.** `Record::Select` carries the slot, the renderer
        // and the instant alone — *"half way to renderer 2" does not name a
        // picture* — and `Deck::schedule_selection` is what it decodes to: a
        // later selection on a slot replaces the earlier one, which is
        // `Deck::schedule`'s own rule for a fade.
        //
        // **The renderer is not checked here and is checked at the beat.** The
        // Set in a slot can change under a hot swap between the schedule and
        // the instant, so a selection that no longer names a renderer is
        // dropped where it is applied. The *slot* is checked, by `held`,
        // because a deck does not change size.
        //
        // **It says nothing about a slot that overdraws**, and that is carried
        // rather than refused: `Record::Select` names an edge into the Set's
        // L5 and a slot built without a composite fold has none, so refusing
        // it would make a replay fail on a line describing a performance that
        // happened.
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = held(deck, slot.0)?;
            deck.schedule_selection(Selection::new(slot.index(), renderer as usize, start));
            Some(format!(
                "  renderer: deck {} -> SelectRenderer {{ renderer: {renderer} }} -> \
                 Record::Select {{ start: {start:.3} }} -> deck.selections_on({slot}) = {} \
                 armed, landing on the beat grid",
                deck_letter(slot.0),
                deck.selections_on(slot).count()
            ))
        }
        // **The session's grid, and the one record here that names no slot and
        // touches no deck control at all.** `Record::Tempo` is a correction —
        // a tempo, a phase shift and how much the estimate behind it was
        // believed — and `karakuri_environment::audio::apply_tempo` is *"the
        // only way a correction reaches the oscillator, live or on replay"*.
        // So this arm is the live half of that sentence, and it is one call
        // rather than a `signals.correct` beside it for exactly the reason
        // that function says so of itself.
        //
        // **The signals are copied out of the deck and back in**, which is
        // what `Deck::signals` and `set_signals` are for and is
        // [`measure_audio`]'s own line: the bus is a `Copy` value and the deck
        // is the model of record for it.
        //
        // **What reaches here today is `Operation::SetFreeRunTempo` and
        // nothing else**, which is *what the grid runs at with nothing driving
        // it* — the one control on this panel that works in the state this
        // program actually runs in, where there is no device and `tapped` and
        // `scaled` both refuse because there is no room. A tap and an octave
        // end in this same record and do **not** come through here: theirs is
        // the beat lock's answer and `written` cannot build it (ADR-0278), so
        // they are applied where they are computed.
        //
        // **The confidence is not printed and the shift is.** A hand-named
        // tempo carries `shift: 0.0` and `confidence: 0.0` — `Record::Tempo`'s
        // own words for a free-running tempo being stated — and a `0.0`
        // confidence beside a tempo an operator just chose would read as *this
        // is not believed*, which is the opposite of what it means. The shift
        // is printed because a beat that did not move is the claim this arm
        // makes.
        Record::Tempo { bpm, shift, .. } => {
            let mut signals = *deck.signals();
            karakuri_environment::audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            Some(format!(
                "  tempo: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} -> \
                 deck.signals().oscillator().bpm() = {:.1}, from now on and without moving a \
                 beat that has already happened",
                deck.signals().oscillator().bpm()
            ))
        }
        _ => None,
    }
}

/// **The reading an operation's record needs, taken off the deck it names.**
///
/// [`written`] builds `Record::Mask` **whole** — a shape, an angle, a position
/// and a softness — out of an operation that names two of the four, and the
/// other two come from a reading of the mask that is running (ADR-0201). This
/// is that reading, and it is the harness's because the deck is
/// (ADR-0156, ADR-0194).
///
/// **`Current::default()` is *I read nothing*, and it is still the answer for
/// four of this panel's emitting controls**: a gain, an opacity, a blend
/// mode and a residency each carry everything their record carries, so handing
/// a reading in would be this file inventing a value. The mask mini is one of
/// those that need one, and it needs it for the deck the operation *names*
/// rather than for the deck the pointer is over — which is `Reading::Mask`'s
/// own wording and the reason this takes the operation and not a slot.
///
/// **The two look controls are two of the other three**, and they read one
/// thing between them: the look that is running. Each names a third of
/// `Record::Look` and the other two thirds come from here — which is
/// [`Reading::Look`]'s own wording and the reason the reading is taken for the
/// operation rather than per control.
///
/// **The scrub's two arrows are the fourth**, and the reading they take is the
/// one thing on this list that is not a completion: see the arm.
///
/// **The sync chip and the anchor are the fifth and sixth**, and they read the
/// one thing here that belongs to no deck: the session tempo. That arm used to
/// be absent and the two controls used to print a question instead of moving
/// anything — see [`unwritten`] for what the question turned out to be.
///
/// **The softness is read back**, where `karakuri-cli`'s `mix::current_mask`
/// substitutes its own `MASK_SOFTNESS`: that program writes wipes and has a
/// softness of its own to write, and this window has never written one. What
/// is read back here is therefore what is actually on the slot, and reading it
/// back is what stops a press rewriting it — the same argument the angle's is,
/// one field along.
///
/// **The `go` capsule is the last of them and it is the one that reads
/// three**: a wipe is written against the transition settings, the mask of the
/// deck arriving and where that deck already sits in the mix. The first is the
/// console's own — `settings` is what [`View::transition`] holds and what the
/// row's three pills move — and the other two are the deck's, taken for the
/// `to` slot and never for the `from`, which is `Current::mask`'s own wording:
/// everything a wipe writes is about the deck arriving.
///
/// A slot the deck has not got answers `None`, and [`written`] then says the
/// reading was owed rather than indexing something that is not there — the
/// guard [`apply`] has, at the other end of the same press.
pub(crate) fn reading(
    operation: &Operation,
    deck: &Deck,
    look: &Look,
    chain: &[karakuri_engine::SlotSpec],
    settings: TransitionSettings,
) -> Current {
    // **The whole chain, for whichever pass was asked for.** The look arm's
    // argument one bay along: `Record::MasterChain` needs all four numbers and
    // each row's press carries one pass, so the running chain is handed in and
    // `written` takes the rows the press did not name.
    let master_chain = match *operation {
        Operation::SetFeedback { .. }
        | Operation::SetBloom { .. }
        | Operation::SetRgbShift { .. } => Some(mix::current_chain(chain)),
        _ => None,
    };
    let look = match *operation {
        // **Two thirds of the record, for whichever third was asked for.**
        // `SetTonemap` carries an operator and `SetExposure` a level, and
        // `Record::Look` needs all three — so the running look is handed in
        // and `written` takes the two the press did not name. That is
        // ADR-0192's argument executable in this file: the operation carries
        // what a surface can say and the translator completes the record.
        // `white_point` is on no surface at all, so it survives every press by
        // arriving here and going straight back out.
        Operation::SetTonemap { .. } | Operation::SetExposure { .. } => {
            Some(karakuri_operation_record::Look {
                tonemap: mix::tonemap(look.op),
                exposure: look.exposure,
                white_point: look.white_point,
            })
        }
        _ => None,
    };
    // **The scrub is the third reading, and it is the only one that is
    // relative.** `Record::Transport` is absolute — a sync mode, an anchor and
    // a scrub position — and `ScrubDeck` names an amount, so the record is
    // where the slot already is plus what was asked for. The reading is
    // therefore not a completion of a record the way the look's and the mask's
    // are: it is the left-hand side of an addition, and without it the
    // conversion answers `Owed(NotRead(Transport))` rather than starting a
    // deck's scrub from zero.
    //
    // **All three fields, because the record is written whole.** A scrub that
    // wrote a position without the mode and the anchor beside it *"would
    // replay a slot onto a grid it was never on"* —
    // `karakuri_operation_record::Transport` says so at its own definition —
    // and the two it does not touch survive the press by arriving here and
    // going straight back out, which is the white point's arrangement one
    // reading up.
    let transport = match *operation {
        Operation::ScrubDeck { deck: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let transport = deck.transport(slot);
                karakuri_operation_record::Transport {
                    sync: mix::sync(transport.sync()),
                    anchor_bpm: transport.anchor_bpm(),
                    scrub_beats: transport.scrub_beats(),
                }
            })
        }
        _ => None,
    };
    // **The tempo the room is going at, and nothing about a slot.** Engaging
    // a sync mode anchors the slot at the session tempo so that the picture
    // does not move at the instant it goes on the grid, which is
    // `karakuri_engine::transport::Transport::engaged`'s policy and the whole
    // of what this reading is for. `mix::current_tempo` takes the oscillator
    // rather than an `f32`, so this window cannot hand in a tempo the session
    // never ran at — and it reads the grid rather than a clock, which is what
    // lets the record be replayed
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    //
    // **No slot check, unlike the three below.** The tempo is the session's,
    // so a `SetSync` naming a slot the deck has not got is a record with a slot
    // out of range rather than a reading that could not be taken, and
    // `apply`'s own guard is what says so at the other end of the press.
    let tempo = match *operation {
        Operation::SetSync { .. } => Some(mix::current_tempo(deck.signals().oscillator())),
        _ => None,
    };
    // **Three operations read this and one of them names two decks.** A wipe
    // writes `Record::Mask` for the deck *arriving* — twice, at the front's
    // present position and then at 0 — so the mask handed over is `to`'s and
    // `from` is read for nothing at all. Handing in the covered deck's would
    // put somebody else's soft edge on the front that is about to cross the
    // frame, which is `Current::mask`'s own sentence and `karakuri-cli`'s
    // `Live::operate` arm exactly.
    //
    // **`SetMaskPosition` was missing from this list until 2026-09-10**, and
    // it is the one arm ADR-0334 recorded as a defect rather than a scope:
    // both halves of a mask write `Record::Mask` whole, so a position needs
    // the shape, the angle and the soft edge it does not name, and without
    // this `written` answered `Owed(NotRead(Mask))` and nothing moved. It was
    // reachable from a mapped controller before it was reachable from a model,
    // so the hole was a MIDI knob that did nothing as well as a call that
    // could not be accepted. `karakuri-cli`'s arm has always named all three,
    // which is what makes this a slip in one file rather than a decision.
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. }
        | Operation::SetMaskPosition { deck: slot, .. }
        | Operation::Wipe { to: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    // **Every field named, and no `..Current::default()` behind them.** Every
    // reading the type carries is answered here, so a fill would be dead — and
    // the day it grows one more, this stops compiling and somebody has to say
    // whether this window can take it, rather than a `None` arriving silently.
    //
    // **The count that used to be in this sentence is gone rather than
    // corrected.** It said four where there are three and named a fifth that
    // would be a fourth, which is a figure nothing checks going stale in the
    // one comment whose whole argument is that the compiler does the checking.
    //
    // **The transition settings used to be answered `None` here, on the
    // grounds that this panel drew no control that set any of them.** That
    // sentence was true of a window with no transition row in it and is not
    // true of this one: the row is drawn, its three pills emit
    // `Operation::SetTransition`, and `View::transition` is the model of
    // record for what they arrive at. So the reading is taken, and it is the
    // one here that is read off the *console* rather than off the deck —
    // a quantum, a length and a wipe shape are a surface's setting deciding
    // what the next move means, and this surface now holds one.
    //
    // **Four operations, which is every one that schedules a move**, and it
    // is `karakuri-cli`'s `Live::operate` arm exactly: only the wipe has a
    // control on this panel today, and the other three are answered because
    // what the reading *is* does not depend on which surface asked. A
    // conversion that came back `Owed(NotRead(Transition))` for a fade the
    // day a fader learned to schedule one would be this arm having to be
    // found again.
    //
    // `mix::current_transition` takes the oscillator and the quantum rather
    // than an instant, so the start is
    // `karakuri_engine::transition::quantise`'s answer and this file cannot
    // hand in a beat the grid was never on — the arrangement `mix::current_tempo`
    // is in one reading up
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    // `mix::FADE_CURVE` is the shape every scheduled fade takes, and it is
    // asked for by name rather than spelled here because two surfaces easing
    // one fade differently is a value a replay carries.
    let transition = match *operation {
        Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::SelectRenderer { .. }
        | Operation::Wipe { .. } => Some(mix::current_transition(
            deck.signals().oscillator(),
            settings.quantum,
            settings.length,
            mix::FADE_CURVE,
            mask_kind(settings.kind),
            settings.angle,
        )),
        _ => None,
    };
    // **Where the deck a wipe is arriving on already sits in the mix**, and
    // the one reading here taken so that a record can be left *out* rather
    // than written. A wipe puts that deck under `over` and on air, and both
    // are a state it may be in already: under `add` the same gesture is a wipe
    // *on* rather than a wipe *over*, a different picture and a legitimate
    // one, so the mode is left where the operator put it. The condition is the
    // conversion's and what it needs to hold it is this
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    //
    // **What the deck reports rather than what it was asked for**, which is
    // `Deck::residency`'s answer: the governor may hold a slot below the
    // request, and what a wipe needs to know is whether the put-on-air it is
    // about to write would change anything.
    let mix = match *operation {
        Operation::Wipe { to: slot, .. } => EngineSlot::new(slot, deck.slot_count())
            .map(|slot| mix::current_mix(deck.blend(slot), deck.residency(slot))),
        _ => None,
    };
    Current {
        look,
        master_chain,
        mask,
        transport,
        tempo,
        transition,
        mix,
    }
}

/// **The master chain, as the console reads it** — a level's reading rather
/// than the level (ADR-0156), and the one place the engine's `Chain` becomes
/// the panel's.
///
/// **Not `mix::current_chain`, and the two are not the same reading.** That one
/// answers the *conversion* — what `written` completes a record from, in the
/// engine's own amounts — and this one answers a *fader*, in track positions.
/// The crossing they share, engine cut to vocabulary cut, is `mix::cut` and is
/// made once.
///
/// **The feedback amount arrives as a track position**, `[0, 1]`, where the
/// engine holds `[0, 0.95]`: a fader draws where it is along its own travel,
/// and `Knob::Feedback` multiplies back by `Feedback::MAX` on the way out. The
/// other two are `[0, 1]` at both ends and pass through.
pub(crate) fn chain_view(chain: &[karakuri_engine::SlotSpec]) -> view::Chain {
    // **The three rows read the three shipped slots**, which is
    // `mix::current_chain`'s own arrangement and is here so the reading is made
    // once: a row whose procedure is not in the chain reads zero, which is the
    // honest reading of *this pass is not running*. The rows retire in M5.16's
    // second pass (ADR-0340 §7) and this function goes with them.
    let running = mix::current_chain(chain);
    view::Chain {
        feedback: running.feedback.amount / karakuri_operation::Feedback::MAX,
        cut: running.feedback.cut,
        bloom: running.bloom,
        rgb_shift: running.rgb_shift,
    }
}

/// **The engine's mask shape, as the vocabulary's** — [`blend_mode`]'s
/// function one control along, and the one place these two lists are made to
/// agree.
///
/// A match, so the day a fourth `MaskKind` lands in the engine this stops
/// compiling rather than reading a shape the vocabulary cannot name into a
/// record that has to name one. The mirror image of it is `view::Mixer::mask`,
/// which turns the console's own word into the same vocabulary — three names
/// for three shapes, which is the cost `karakuri-operation` pays for depending
/// on nothing (P-0090).
pub(crate) fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
    }
}

/// **The vocabulary's mask shape, as the engine's** — [`wipe_kind`] read the
/// other way, and the two are a pair rather than one function because the
/// crossing happens in both directions in this file.
///
/// The console holds the shape the *next* wipe takes as a
/// [`karakuri_operation::WipeKind`] — `TransitionSetting::WipeShape` is what
/// its pill emits — and `mix::current_transition` takes the engine's, so a
/// wipe's front crosses here on its way to the reading. It crosses back
/// inside that function, through `mix`'s own `wipe_kind`, which is the one
/// place `karakuri-environment` makes the two lists agree: what a record
/// carries is the vocabulary's word either way, and this round trip is the
/// price of a signature that speaks the engine's types to a caller holding
/// them (`karakuri-cli` is that caller).
///
/// A match for [`wipe_kind`]'s reason, so a fourth shape on either side stops
/// the build here rather than at a record naming a shape nothing can read.
pub(crate) fn mask_kind(kind: karakuri_operation::WipeKind) -> MaskKind {
    match kind {
        karakuri_operation::WipeKind::None => MaskKind::None,
        karakuri_operation::WipeKind::Linear => MaskKind::Linear,
        karakuri_operation::WipeKind::Radial => MaskKind::Radial,
    }
}

/// **What a frame has to fit in on this window**: the display's refresh
/// interval, in milliseconds — the `/16.6` in the mock's transport, at the
/// 60 Hz it was drawn against.
///
/// **It is the refresh interval because that is what this window is held to.**
/// The surface is `PresentMode::Fifo`, so a frame that takes longer than one
/// interval to build is a frame that misses a vsync, and every millisecond
/// under it is the headroom the mock's own tooltip is about. `Cost::wait` is
/// the other side of the same number: at 60 Hz most of the frame is spent
/// blocked in `get_current_texture` waiting for it.
///
/// **It is not `karakuri_engine`'s `DEFAULT_BUDGET_MS`**, which is 20 and is a
/// different budget with the same word on it: that one is what a *candidate
/// Set* has to hold to survive a hot swap, measured offscreen at a fixed size
/// and judged on a median. The mock's tooltip runs the two together — *"12.4
/// of 16.6 — there is headroom. A candidate that cannot hold this is rolled
/// back on its own"* — and they are two numbers. This row draws the one the
/// frame is actually against.
///
/// `None` where `winit` will not say, which is a monitor it cannot name or a
/// mode with no refresh rate on it. The row then draws the frame time and no
/// budget, rather than a plausible 16.6 nothing measured.
///
/// **Read once, when the window opens.** A window dragged onto a 120 Hz
/// display keeps the interval it opened on, which is a real limitation and is
/// the price of not asking the platform for a monitor handle sixty times a
/// second.
pub(crate) fn budget_ms(window: &Window) -> Option<f32> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    match millihertz > 0 {
        true => Some(1.0e6 / millihertz as f32),
        false => None,
    }
}

/// A `.kir` off disk, parsed and checked — the two stages `Set::build` wants a
/// `Checked` from, and no more. `karakuri-cli`'s `compile::load` is the same
/// two with a cost estimate and a source it keeps; neither is wanted here.
/// **How many elements the geometry runs at**, which is the L1's own
/// declaration and not a number written here.
///
/// `capacity [min, max] = default` is in the file and `Set::build` takes a
/// number, so somebody has to read one across. This used to be a
/// `const CAPACITY: u32 = 262144` — `drift_shell.kir`'s declared default,
/// transcribed, which was fine while that was the only file this could load and
/// silently wrong the moment it took a path: a procedure written for 131072
/// would have run at 262144 and nothing would have said so.
///
/// **The fallback is not the answer, it is the arm that cannot happen.**
/// `karakuri_ir::DEFAULT_CAPACITY` is what is left when *nothing* declared one,
/// and `check_header` requires a `capacity` on every L1 — so a `Checked` that
/// passed always carries one and this `map_or` is the shape of the seam type
/// rather than a decision. Pointing the whole thing at `DEFAULT_CAPACITY` would
/// run every procedure at 262144 whatever it declared, which is the defect
/// `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none` is run
/// against.
///
/// This is `karakuri-cli`'s `capacity_for` with no `--capacity` to override it,
/// and `karakuri_environment::watch`'s rebuild is the same line again. There is
/// no `--capacity` here on purpose: see [`USAGE`].
pub(crate) fn capacity_of(l1: &karakuri_ir::typed::Checked) -> u32 {
    l1.capacity
        .map_or(karakuri_ir::DEFAULT_CAPACITY, |declared| declared.default)
}

/// A rectangle in logical pixels, in physical ones. **Both of the engine's
/// textures are sized from this and from nothing else** — the picture from its
/// region, deck A's preview from its cell — which is what makes each of them
/// the size of what it is drawn into rather than of the window. Which
/// rectangle each one gets is [`aims`]; this is the *at what size* half, and
/// [`Presented::aim`] is the one place the two meet.
pub(crate) fn physical(rect: egui::Rect, scale: f32) -> (u32, u32) {
    (
        ((rect.width() * scale).round() as u32).max(1),
        ((rect.height() * scale).round() as u32).max(1),
    )
}
