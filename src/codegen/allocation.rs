#[derive(Clone, Debug)]
pub struct Reservation {
    pub begin: i32,
    pub end: i32,
    pub name: String,
}
#[derive(Clone, Debug)]
pub struct AllocationRegion {
    pub name: String,
    pub bottom: i32,
    pub top: i32,
    pub next: i32,
    pub reservations: Vec<Reservation>,
}
impl AllocationRegion {
    pub fn new(name: impl Into<String>, bottom: i32, top: i32) -> Self {
        Self {
            name: name.into(),
            bottom,
            top,
            next: bottom,
            reservations: Vec::new(),
        }
    }
    pub fn reserve(&mut self, name: &str, address: i32, size: i32) -> Result<(), String> {
        if size <= 0 {
            return Ok(());
        }
        let end = address.wrapping_add(size).wrapping_sub(1);
        for r in &self.reservations {
            if address <= r.end && end >= r.begin {
                return Err(format!(
                    "fixed allocation '{name}' at {:04X}-{:04X} overlaps fixed allocation '{}' at {:04X}-{:04X}",
                    address & 65535,
                    end & 65535,
                    r.name,
                    r.begin & 65535,
                    r.end & 65535
                ));
            }
        }
        if address < self.next {
            return Err(format!(
                "fixed allocation '{name}' at {:04X}-{:04X} overlaps prior automatic allocation in {} ending at {:04X}",
                address & 65535,
                end & 65535,
                self.name,
                (self.next - 1) & 65535
            ));
        }
        self.reservations.push(Reservation {
            begin: address,
            end,
            name: name.into(),
        });
        self.reservations.sort_by_key(|r| r.begin);
        Ok(())
    }
    pub fn allocate(&mut self, size: i32, alignment: i32) -> Option<i32> {
        let mut address = align_up(self.next, alignment);
        loop {
            let end = address.wrapping_add(size).wrapping_sub(1);
            let Some(reservation) = self
                .reservations
                .iter()
                .find(|r| address <= r.end && end >= r.begin)
            else {
                break;
            };
            address = align_up(reservation.end.wrapping_add(1), alignment)
        }
        if address.wrapping_add(size).wrapping_sub(1) > self.top {
            return None;
        }
        self.next = address.wrapping_add(size);
        Some(address)
    }
}
pub fn align_up(value: i32, alignment: i32) -> i32 {
    if alignment <= 1 {
        value
    } else {
        let mask = alignment - 1;
        value.wrapping_add(mask) & !mask
    }
}
#[derive(Clone, Debug)]
pub struct Allocator {
    pub regions: Vec<AllocationRegion>,
    frame_peak: Option<[i32; 3]>,
}
impl Allocator {
    pub fn new(fixed_stack: bool, limit: i32) -> Self {
        let mut regions = vec![
            AllocationRegion::new("HRAM", 0xff80, 0xfffe),
            AllocationRegion::new("WRAM0", 0xc000, 0xcfff),
            AllocationRegion::new("WRAM1", 0xd000, 0xdfff),
            AllocationRegion::new("OAM", 0xfe00, 0xfe9f),
        ];
        for bank in 2..=7 {
            regions.push(AllocationRegion::new(
                format!("WRAMX[{bank}]"),
                0xd000,
                0xdfff,
            ))
        }
        let region = if fixed_stack { 1 } else { 2 };
        regions[region].top = regions[region].top.min(limit);
        Self {
            regions,
            frame_peak: None,
        }
    }
    pub fn bank_region(bank: i32) -> usize {
        if (2..=7).contains(&bank) {
            bank as usize + 2
        } else {
            2
        }
    }
    pub fn allocate(&mut self, region: usize, size: i32, align: i32) -> Option<i32> {
        let result = self.regions[region].allocate(size, align)?;
        if region < 3 {
            if let Some(peak) = &mut self.frame_peak {
                peak[region] = peak[region].max(self.regions[region].next)
            }
        }
        Some(result)
    }
    pub fn snapshot(&self) -> [i32; 3] {
        [
            self.regions[0].next,
            self.regions[1].next,
            self.regions[2].next,
        ]
    }
    pub fn restore(&mut self, saved: [i32; 3]) {
        for (region, next) in self.regions.iter_mut().take(3).zip(saved) {
            region.next = next
        }
    }
    pub fn begin_frame(&mut self) {
        self.frame_peak = Some(self.snapshot())
    }
    pub fn commit_frame(&mut self) {
        if let Some(peak) = self.frame_peak.take() {
            for (region, peak) in self.regions.iter_mut().take(3).zip(peak) {
                region.next = region.next.max(peak)
            }
        }
    }
    pub fn reserve_fixed(&mut self, name: &str, address: i32, size: i32) -> Result<(), String> {
        if size <= 0 {
            return Ok(());
        }
        let end = address.wrapping_add(size).wrapping_sub(1);
        let region = if address >= 0xc000 && end <= 0xcfff {
            Some(1)
        } else if address >= 0xd000 && end <= 0xdfff {
            Some(2)
        } else if address >= 0xff80 && end <= self.regions[0].top {
            Some(0)
        } else if address >= 0xfe00 && end <= self.regions[3].top {
            Some(3)
        } else {
            None
        };
        if let Some(region) = region {
            self.regions[region].reserve(name, address, size)
        } else {
            Ok(())
        }
    }
}
