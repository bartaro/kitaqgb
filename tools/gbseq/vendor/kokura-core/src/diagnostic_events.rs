use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// SARAKURA-facing runtime diagnostic event.
///
/// This is intentionally much smaller than a CPU/PPU trace.  It records only
/// violations or already-aggregated observations that SARAKURA can correlate
/// with KITAQGB build metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub schema: String,
    pub schema_version: u32,
    pub event_id: String,
    pub event_type: String,
    pub severity: String,
    pub frame: u64,
    pub scanline: Option<u8>,
    pub dot: Option<u32>,
    pub pc: Option<String>,
    pub bank: Option<u16>,
    pub addr: Option<String>,
    pub value: Option<String>,
    pub access_kind: Option<String>,
    pub ppu_mode: Option<u8>,
    pub lcdc: Option<String>,
    pub stat: Option<String>,
    pub dma_state: Option<String>,
    pub summary_key: String,
    pub count: u64,
    pub first_seen: u64,
    pub last_seen: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sample_events: Vec<DiagnosticEventSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticEventSample {
    pub frame: u64,
    pub scanline: Option<u8>,
    pub dot: Option<u32>,
    pub pc: Option<String>,
    pub bank: Option<u16>,
    pub addr: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticEventEmitter {
    enabled: bool,
    next_id: u64,
    max_unique_events: usize,
    events: Vec<DiagnosticEvent>,
    index_by_summary_key: BTreeMap<String, usize>,
}

impl Default for DiagnosticEventEmitter {
    fn default() -> Self {
        Self {
            enabled: false,
            next_id: 1,
            max_unique_events: 4096,
            events: Vec::new(),
            index_by_summary_key: BTreeMap::new(),
        }
    }
}

impl DiagnosticEventEmitter {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn clear(&mut self) {
        self.next_id = 1;
        self.events.clear();
        self.index_by_summary_key.clear();
    }

    pub fn events(&self) -> &[DiagnosticEvent] {
        &self.events
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_gb_event(
        &mut self,
        event_type: &str,
        severity: &str,
        frame: u64,
        scanline: Option<u8>,
        dot: Option<u32>,
        pc: Option<u16>,
        bank: Option<u16>,
        addr: Option<u16>,
        value: Option<u8>,
        access_kind: Option<&str>,
        ppu_mode: Option<u8>,
        lcdc: Option<u8>,
        stat: Option<u8>,
        dma_active: Option<bool>,
    ) {
        if !self.enabled {
            return;
        }
        let pc_hex = pc.map(|pc| format!("0x{pc:04X}"));
        let addr_hex = addr.map(|addr| format!("0x{addr:04X}"));
        let value_hex = value.map(|value| format!("0x{value:02X}"));
        let summary_key = format!(
            "{}+pc={}+bank={}+addr={}+access={}",
            event_type,
            pc_hex.as_deref().unwrap_or("?"),
            bank.map(|b| b.to_string())
                .unwrap_or_else(|| "?".to_string()),
            addr_hex.as_deref().unwrap_or("?"),
            access_kind.unwrap_or("?")
        );
        if let Some(idx) = self.index_by_summary_key.get(&summary_key).copied() {
            let event = &mut self.events[idx];
            event.count = event.count.saturating_add(1);
            event.last_seen = frame;
            if event.sample_events.len() < 4 {
                event.sample_events.push(DiagnosticEventSample {
                    frame,
                    scanline,
                    dot,
                    pc: pc_hex,
                    bank,
                    addr: addr_hex,
                    value: value_hex,
                });
            }
            return;
        }
        if self.events.len() >= self.max_unique_events {
            return;
        }
        let event_id = format!("evt_{:06}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let event = DiagnosticEvent {
            schema: "kokura-diagnostic-event".to_string(),
            schema_version: 3,
            event_id,
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            frame,
            scanline,
            dot,
            pc: pc_hex.clone(),
            bank,
            addr: addr_hex.clone(),
            value: value_hex.clone(),
            access_kind: access_kind.map(str::to_string),
            ppu_mode,
            lcdc: lcdc.map(|value| format!("0x{value:02X}")),
            stat: stat.map(|value| format!("0x{value:02X}")),
            dma_state: dma_active.map(|active| if active { "active" } else { "idle" }.to_string()),
            summary_key: summary_key.clone(),
            count: 1,
            first_seen: frame,
            last_seen: frame,
            sample_events: Vec::new(),
        };
        let idx = self.events.len();
        self.events.push(event);
        self.index_by_summary_key.insert(summary_key, idx);
    }
}
