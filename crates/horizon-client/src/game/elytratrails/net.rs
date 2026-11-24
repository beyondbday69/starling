use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

use super::core as m;
use crate::game::Game;

pub const CHANNEL: &str = "elytratrails:color";
const ENTRY: usize = 18;
const SEND_PERIOD: u64 = 40;

type Style = (u8, u8);

#[derive(Default)]
struct Net {
    ticks: u64,
    server: HashMap<u128, Style>,
    dirty: bool,
    client: HashMap<u128, Style>,
    by_id: HashMap<u32, Style>,
}

static NET: Mutex<Option<Net>> = Mutex::new(None);

fn with_net<R>(f: impl FnOnce(&mut Net) -> R) -> R {
    let mut guard = NET.lock().unwrap_or_else(PoisonError::into_inner);
    f(guard.get_or_insert_with(Net::default))
}

fn valid(colour: u8, pattern: u8) -> bool {
    usize::from(colour) < m::COLORS.len() && usize::from(pattern) < m::PATTERNS.len()
}

fn encode(styles: &HashMap<u128, Style>) -> Vec<u8> {
    let mut out = Vec::with_capacity(styles.len() * ENTRY);
    for (uuid, (colour, pattern)) in styles {
        out.extend_from_slice(&uuid.to_be_bytes());
        out.push(*colour);
        out.push(*pattern);
    }
    out
}

fn decode(data: &[u8]) -> HashMap<u128, Style> {
    data.chunks_exact(ENTRY)
        .filter(|chunk| valid(chunk[16], chunk[17]))
        .map(|chunk| {
            let mut uuid = [0u8; 16];
            uuid.copy_from_slice(&chunk[..16]);
            (u128::from_be_bytes(uuid), (chunk[16], chunk[17]))
        })
        .collect()
}

pub fn style_of(id: u32) -> Option<(usize, usize)> {
    with_net(|net| {
        net.by_id
            .get(&id)
            .map(|(colour, pattern)| (usize::from(*colour), usize::from(*pattern)))
    })
}

impl Game<'_> {
    pub(crate) fn elytratrails_own_uuid(&self) -> u128 {
        if let Some(r) = self.remote.as_ref() {
            return r.profile();
        }
        self.lan_acting_uuid.unwrap_or_else(|| {
            self.level.player_uuid.map_or(0, |u| {
                u.iter()
                    .fold(0u128, |a, &w| (a << 32) | u128::from(w as u32))
            })
        })
    }

    pub(crate) fn elytratrails_net_from_client(
        &mut self,
        player: u128,
        channel: &str,
        data: &[u8],
    ) {
        if channel != CHANNEL || data.len() < 2 || !valid(data[0], data[1]) {
            return;
        }
        let style = (data[0], data[1]);
        with_net(|net| {
            let changed = net.server.insert(player, style) != Some(style);
            net.dirty |= changed;
        });
    }

    pub(crate) fn elytratrails_net_from_server(&mut self, channel: &str, data: &[u8]) {
        self.elytratrails_client_message(channel, data);
    }

    pub(crate) fn elytratrails_net_server_tick(&mut self) {
        let active: HashSet<u128> = self
            .native_server_players()
            .iter()
            .map(|p| p.uuid())
            .collect();
        let changed = with_net(|net| {
            let dropped = net.server.len();
            net.server.retain(|uuid, _| active.contains(uuid));
            let pruned = net.server.len() != dropped;
            let send = net.dirty || pruned;
            net.dirty = false;
            send
        });
        if !changed {
            return;
        }
        let table = with_net(|net| encode(&net.server));
        for player in self.native_server_players() {
            let _ = self.native_net_send_to_player(player.uuid(), CHANNEL, &table);
        }
    }

    pub(crate) fn elytratrails_client_message(&mut self, channel: &str, data: &[u8]) {
        if channel != CHANNEL {
            return;
        }
        let styles = decode(data);
        with_net(|net| net.client = styles);
    }

    pub(crate) fn elytratrails_net_tick(&mut self) {
        if self.remote.is_none() && self.lan.is_none() {
            with_net(|net| {
                if !(net.client.is_empty() && net.server.is_empty() && net.by_id.is_empty()) {
                    net.client.clear();
                    net.server.clear();
                    net.by_id.clear();
                    net.dirty = false;
                }
            });
            return;
        }
        let settings = self.elytratrails_settings();
        let style = (settings.color as u8, settings.pattern as u8);
        let own = self.elytratrails_own_uuid();
        let hosting = self.lan.is_some() && self.remote.is_none();
        let send = self.remote.is_some();
        let send_now = with_net(|net| {
            net.ticks += 1;
            if hosting && own != 0 {
                let changed = net.server.insert(own, style) != Some(style);
                net.dirty |= changed;
            }
            let mut by_id = HashMap::new();
            if let Some(remote) = self.remote.as_ref() {
                for (uuid, value) in &net.client {
                    if let Some(id) = remote.player_entity(*uuid) {
                        by_id.insert(id, *value);
                    }
                }
            }
            if let Some(host) = self.lan.as_ref() {
                for guest in &host.hub.guests {
                    if let Some(value) = net.server.get(&guest.uuid) {
                        by_id.insert(guest.entity_id as u32, *value);
                    }
                }
            }
            net.by_id = by_id;
            send && net.ticks % SEND_PERIOD == 0
        });
        if send_now {
            let _ = self.native_net_send_to_server(CHANNEL, &[style.0, style.1]);
        }
    }
}
