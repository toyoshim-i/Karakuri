use super::*;

/// One slot's watcher, and where it is pointed.
pub(crate) struct Aiming {
    /// The other end of `watch::Watch::aimed_by`'s channel.
    pub(crate) aim: std::sync::mpsc::Sender<watch::Aim>,
    /// Where that watcher is pointed, kept in step with what has been sent.
    pub(crate) at: watch::Aim,
    /// Handle where this slot's files are published for external readers.
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Slot index corresponding to the deck letter.
    pub(crate) slot: usize,
}

impl Aiming {
    /// A watcher, and the handle where this slot's files are published.
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

    /// Say where this watcher is pointed, from the aim and from nothing else.
    pub(crate) fn publish(&self) {
        self.pointing.re_point(self.slot, &self.at);
    }

    /// Point the watcher at what it is already looking at, with one field changed.
    pub(crate) fn changed(&mut self, change: impl FnOnce(&mut watch::Aim)) -> Result<(), ()> {
        change(&mut self.at);
        self.publish();
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }

    /// Re-aims the slot with updated graph wiring edges.
    pub(crate) fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.changed(|at| at.edges = edges)
    }

    /// Point it at something else entirely, keeping the aim that was sent.
    pub(crate) fn re_point(&mut self, aim: watch::Aim) -> Result<(), ()> {
        self.at = aim;
        self.publish();
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// One aim, restated for transmission across threads.
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
        set: set.clone(),
    }
}
