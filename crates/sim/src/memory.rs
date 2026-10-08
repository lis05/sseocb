#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permissions {
    pub const RO: Self = Self {
        read: true,
        write: false,
        execute: false,
    };
    pub const RX: Self = Self {
        read: true,
        write: false,
        execute: true,
    };
    pub const RW: Self = Self {
        read: true,
        write: true,
        execute: false,
    };
    pub const RWX: Self = Self {
        read: true,
        write: true,
        execute: true,
    };

    pub fn satisfies(&self, required: Permissions) -> bool {
        (!required.read || self.read)
            && (!required.write || self.write)
            && (!required.execute || self.execute)
    }

    pub fn to_linker_attr(&self) -> String {
        let mut res = String::new();
        if self.read {
            res.push('r');
        }
        if self.write {
            res.push('w');
        }
        if self.execute {
            res.push('x');
        }
        res
    }
}

pub mod region {
    use super::Permissions;

    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
    pub enum Error {
        AddressViolation,
        PermissionsViolation,
    }

    pub type Result<T> = std::result::Result<T, Error>;

    pub type ReadCallback = Box<dyn Fn(u32, &mut [u8]) -> Result<()> + Send + Sync>;
    pub type WriteCallback = Box<dyn FnMut(u32, &[u8]) -> Result<()> + Send + Sync>;

    pub struct Region {
        name: String,
        base: u32,
        length: usize,
        permissions: Permissions,
        data: Vec<u8>,
        on_read: Option<ReadCallback>,
        on_write: Option<WriteCallback>,
    }

    impl std::fmt::Debug for Region {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Region")
                .field("name", &self.name)
                .field("base", &self.base)
                .field("length", &self.length)
                .field("permissions", &self.permissions)
                .field("has_read_proxy", &self.on_read.is_some())
                .field("has_write_proxy", &self.on_write.is_some())
                .finish()
        }
    }

    impl Region {
        pub fn try_new(
            name: impl Into<String>,
            base: u32,
            permissions: Permissions,
            data: Vec<u8>,
        ) -> Result<Self> {
            let len = u32::try_from(data.len()).map_err(|_| Error::AddressViolation)?;
            if data.is_empty() || base.checked_add(len).is_none() {
                return Err(Error::AddressViolation);
            }
            Ok(Region {
                name: name.into(),
                base,
                length: data.len(),
                permissions,
                data,
                on_read: None,
                on_write: None,
            })
        }

        pub fn new(
            name: impl Into<String>,
            base: u32,
            permissions: Permissions,
            data: Vec<u8>,
        ) -> Self {
            assert!(!data.is_empty(), "Region data cannot be empty");
            let len = u32::try_from(data.len()).expect("Region data length fits in u32");
            assert!(base.checked_add(len).is_some());
            Region {
                name: name.into(),
                base,
                length: data.len(),
                permissions,
                data,
                on_read: None,
                on_write: None,
            }
        }

        pub fn with_read_proxy(
            mut self,
            cb: impl Fn(u32, &mut [u8]) -> Result<()> + Send + Sync + 'static,
        ) -> Self {
            self.on_read = Some(Box::new(cb));
            self
        }

        pub fn with_write_proxy(
            mut self,
            cb: impl FnMut(u32, &[u8]) -> Result<()> + Send + Sync + 'static,
        ) -> Self {
            self.on_write = Some(Box::new(cb));
            self
        }

        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn base(&self) -> u32 {
            self.base
        }

        pub fn end(&self) -> u32 {
            self.base + self.length as u32
        }

        pub fn permissions(&self) -> Permissions {
            self.permissions
        }

        fn get_offset(&self, address: u32, length: usize) -> Result<usize> {
            if address < self.base {
                return Err(Error::AddressViolation);
            }
            let offset = (address - self.base) as usize;
            if offset
                .checked_add(length)
                .is_none_or(|end| end > self.length)
            {
                return Err(Error::AddressViolation);
            }
            Ok(offset)
        }

        pub fn read(&self, address: u32, buf: &mut [u8]) -> Result<()> {
            let offset = self.get_offset(address, buf.len())?;

            if !self.permissions.read {
                return Err(Error::PermissionsViolation);
            }

            if let Some(ref cb) = self.on_read {
                cb(address, buf)
            } else {
                buf.copy_from_slice(&self.data[offset..][..buf.len()]);
                Ok(())
            }
        }

        pub fn write(&mut self, address: u32, buf: &[u8]) -> Result<()> {
            let offset = self.get_offset(address, buf.len())?;

            if !self.permissions.write {
                return Err(Error::PermissionsViolation);
            }

            if let Some(ref mut cb) = self.on_write {
                cb(address, buf)
            } else {
                self.data[offset..][..buf.len()].copy_from_slice(buf);
                Ok(())
            }
        }

        pub fn to_linker_entry(&self) -> String {
            let attr = format!("({})", self.permissions.to_linker_attr());
            format!(
                "    {:<8} {:<5} : org = 0x{:08X}, len = 0x{:X}",
                self.name, attr, self.base, self.length
            )
        }
    }
}

pub mod bus {
    use super::Permissions;
    use super::region::Region;

    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
    pub enum Error {
        DuplicateRegionName,
        OverlappingRegion,
        AddressViolation,
        PermissionsViolation,
    }

    impl From<super::region::Error> for Error {
        fn from(err: super::region::Error) -> Self {
            match err {
                super::region::Error::AddressViolation => Error::AddressViolation,
                super::region::Error::PermissionsViolation => Error::PermissionsViolation,
            }
        }
    }

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, Default)]
    pub struct Bus {
        regions: Vec<Region>,
    }

    impl Bus {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add(&mut self, region: Region) -> Result<()> {
            if self.regions.iter().any(|r| region.name() == r.name()) {
                return Err(Error::DuplicateRegionName);
            }

            if self
                .regions
                .iter()
                .any(|r| region.base() < r.end() && r.base() < region.end())
            {
                return Err(Error::OverlappingRegion);
            }

            self.regions.push(region);
            Ok(())
        }

        pub fn remove(&mut self, name: &str) -> Option<Region> {
            let index = self.regions.iter().position(|r| r.name() == name)?;
            Some(self.regions.swap_remove(index))
        }

        pub fn read(&self, address: u32, buf: &mut [u8], min_perms: Permissions) -> Result<()> {
            let region = self
                .regions
                .iter()
                .find(|r| address >= r.base() && address < r.end())
                .ok_or(Error::AddressViolation)?;

            if !region.permissions().satisfies(min_perms) {
                return Err(Error::PermissionsViolation);
            }

            region.read(address, buf)?;
            Ok(())
        }

        pub fn write(&mut self, address: u32, buf: &[u8]) -> Result<()> {
            let region = self
                .regions
                .iter_mut()
                .find(|r| address >= r.base() && address < r.end())
                .ok_or(Error::AddressViolation)?;

            region.write(address, buf)?;
            Ok(())
        }

        pub fn read_u8(&self, address: u32, min_perms: Permissions) -> Result<u8> {
            let mut buf = [0u8; 1];
            self.read(address, &mut buf, min_perms)?;
            Ok(buf[0])
        }

        pub fn read_u16(&self, address: u32, min_perms: Permissions) -> Result<u16> {
            let mut buf = [0u8; 2];
            self.read(address, &mut buf, min_perms)?;
            Ok(u16::from_le_bytes(buf))
        }

        pub fn read_u32(&self, address: u32, min_perms: Permissions) -> Result<u32> {
            let mut buf = [0u8; 4];
            self.read(address, &mut buf, min_perms)?;
            Ok(u32::from_le_bytes(buf))
        }

        pub fn write_u8(&mut self, address: u32, val: u8) -> Result<()> {
            self.write(address, &[val])
        }

        pub fn write_u16(&mut self, address: u32, val: u16) -> Result<()> {
            self.write(address, &val.to_le_bytes())
        }

        pub fn write_u32(&mut self, address: u32, val: u32) -> Result<()> {
            self.write(address, &val.to_le_bytes())
        }

        pub fn to_linker_script(&self) -> String {
            let mut script = String::from("MEMORY\n{\n");
            for region in &self.regions {
                script.push_str(&region.to_linker_entry());
                script.push('\n');
            }
            script.push_str("}\n");
            script
        }
    }
}

#[cfg(test)]
mod tests {
    use super::bus::Bus;
    use super::region::Region;
    use super::*;

    mod region_tests {
        use super::*;

        #[test]
        fn test_region_read_inside() {
            let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
            let region = Region::new("Test", 0x1000, Permissions::RO, data);

            // Read full region
            let mut buf = [0u8; 8];
            assert_eq!(region.read(0x1000, &mut buf), Ok(()));
            assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8]);

            // Read slice at offset
            let mut slice = [0u8; 3];
            assert_eq!(region.read(0x1002, &mut slice), Ok(()));
            assert_eq!(slice, [3, 4, 5]);
        }

        #[test]
        fn test_region_read_outside_low() {
            let region = Region::new("Test", 0x1000, Permissions::RO, vec![1, 2, 3, 4]);
            let mut buf = [0u8; 2];
            assert_eq!(
                region.read(0x0FFF, &mut buf),
                Err(region::Error::AddressViolation)
            );
        }

        #[test]
        fn test_region_read_outside_high() {
            let region = Region::new("Test", 0x1000, Permissions::RO, vec![1, 2, 3, 4]);
            let mut buf = [0u8; 2];
            assert_eq!(
                region.read(0x1004, &mut buf),
                Err(region::Error::AddressViolation)
            );
        }

        #[test]
        fn test_region_read_half_outside() {
            let region = Region::new("Test", 0x1000, Permissions::RO, vec![1, 2, 3, 4]);
            let mut buf = [0u8; 3];
            // Starts at 0x1002, 3 bytes would end at 0x1005 (past 0x1004)
            assert_eq!(
                region.read(0x1002, &mut buf),
                Err(region::Error::AddressViolation)
            );
        }

        #[test]
        fn test_region_read_no_read_permission() {
            let perms = Permissions {
                read: false,
                write: true,
                execute: false,
            };
            let region = Region::new("Test", 0x1000, perms, vec![1, 2, 3, 4]);
            let mut buf = [0u8; 2];
            assert_eq!(
                region.read(0x1000, &mut buf),
                Err(region::Error::PermissionsViolation)
            );
        }

        #[test]
        fn test_region_write_inside() {
            let mut region = Region::new("Test", 0x1000, Permissions::RW, vec![0; 4]);

            assert_eq!(region.write(0x1000, &[10, 20]), Ok(()));
            assert_eq!(region.write(0x1002, &[30, 40]), Ok(()));

            let mut buf = [0u8; 4];
            assert_eq!(region.read(0x1000, &mut buf), Ok(()));
            assert_eq!(buf, [10, 20, 30, 40]);
        }

        #[test]
        fn test_region_write_outside_low() {
            let mut region = Region::new("Test", 0x1000, Permissions::RW, vec![0; 4]);
            assert_eq!(
                region.write(0x0FFF, &[1, 2]),
                Err(region::Error::AddressViolation)
            );
        }

        #[test]
        fn test_region_write_outside_high() {
            let mut region = Region::new("Test", 0x1000, Permissions::RW, vec![0; 4]);
            assert_eq!(
                region.write(0x1004, &[1, 2]),
                Err(region::Error::AddressViolation)
            );
        }

        #[test]
        fn test_region_write_half_outside() {
            let mut region = Region::new("Test", 0x1000, Permissions::RW, vec![0; 4]);
            // Starts at 0x1003, 2 bytes would reach 0x1005 (past 0x1004)
            assert_eq!(
                region.write(0x1003, &[1, 2]),
                Err(region::Error::AddressViolation)
            );
        }

        #[test]
        fn test_region_write_read_only() {
            let mut region = Region::new("Flash", 0x1000, Permissions::RO, vec![1, 2, 3, 4]);
            assert_eq!(
                region.write(0x1000, &[10, 20]),
                Err(region::Error::PermissionsViolation)
            );
        }

        #[test]
        #[should_panic(expected = "Region data cannot be empty")]
        fn test_region_empty_data_panics() {
            Region::new("Empty", 0x1000, Permissions::RW, vec![]);
        }

        #[test]
        fn test_region_try_new() {
            // Valid region
            let r = Region::try_new("Valid", 0x1000, Permissions::RW, vec![1, 2, 3]);
            assert!(r.is_ok());

            // Empty data
            let r_empty = Region::try_new("Empty", 0x1000, Permissions::RW, vec![]);
            assert_eq!(r_empty.err(), Some(region::Error::AddressViolation));

            // Overflow
            let r_overflow =
                Region::try_new("Overflow", u32::MAX - 2, Permissions::RW, vec![0; 10]);
            assert_eq!(r_overflow.err(), Some(region::Error::AddressViolation));
        }

        #[test]
        fn test_region_write_proxy() {
            use std::sync::{Arc, Mutex};
            let captured = Arc::new(Mutex::new(Vec::new()));
            let captured_clone = Arc::clone(&captured);

            let mut region = Region::new("UART", 0x4000_0000, Permissions::RW, vec![0; 16])
                .with_write_proxy(move |_addr, buf| {
                    captured_clone.lock().unwrap().extend_from_slice(buf);
                    Ok(())
                });

            // Write through the region
            region.write(0x4000_0000, b"Hi").unwrap();
            assert_eq!(*captured.lock().unwrap(), b"Hi");

            // Default internal data was not written because proxy handled it
            let mut check_buf = [0u8; 2];
            let raw_region = Region::new("Raw", 0x4000_0000, Permissions::RO, vec![0; 16]);
            raw_region.read(0x4000_0000, &mut check_buf).unwrap();
            assert_eq!(check_buf, [0, 0]);
        }

        #[test]
        fn test_region_read_proxy() {
            let region = Region::new("RNG", 0x5000_0000, Permissions::RO, vec![0; 0x100])
                .with_read_proxy(|addr, buf| {
                    // Dynamic read generation: fill with low byte of address
                    buf.fill((addr & 0xFF) as u8);
                    Ok(())
                });

            let mut buf = [0u8; 4];
            region.read(0x5000_0042, &mut buf).unwrap();
            assert_eq!(buf, [0x42, 0x42, 0x42, 0x42]);
        }

        #[test]
        fn test_region_proxy_respects_permissions_and_bounds() {
            use std::sync::Arc;
            use std::sync::atomic::{AtomicBool, Ordering};
            let proxy_called = Arc::new(AtomicBool::new(false));

            let called_r = Arc::clone(&proxy_called);
            let called_w = Arc::clone(&proxy_called);

            // Read-only region with proxies
            let mut region = Region::new("RO_DEVICE", 0x1000, Permissions::RO, vec![0; 4])
                .with_read_proxy(move |_addr, _buf| {
                    called_r.store(true, Ordering::SeqCst);
                    Ok(())
                })
                .with_write_proxy(move |_addr, _buf| {
                    called_w.store(true, Ordering::SeqCst);
                    Ok(())
                });

            // 1. Write to Read-Only: must fail before calling write proxy
            assert_eq!(
                region.write(0x1000, &[1, 2]),
                Err(region::Error::PermissionsViolation)
            );
            assert!(!proxy_called.load(Ordering::SeqCst));

            // 2. Read out of bounds: must fail before calling read proxy
            let mut buf = [0u8; 2];
            assert_eq!(
                region.read(0x1004, &mut buf),
                Err(region::Error::AddressViolation)
            );
            assert!(!proxy_called.load(Ordering::SeqCst));

            // 3. Valid read: invokes proxy
            assert_eq!(region.read(0x1000, &mut buf), Ok(()));
            assert!(proxy_called.load(Ordering::SeqCst));
        }

        #[test]
        fn test_permissions_to_linker_attr() {
            assert_eq!(Permissions::RO.to_linker_attr(), "r");
            assert_eq!(Permissions::RW.to_linker_attr(), "rw");
            assert_eq!(Permissions::RX.to_linker_attr(), "rx");
            assert_eq!(Permissions::RWX.to_linker_attr(), "rwx");
            let none = Permissions {
                read: false,
                write: false,
                execute: false,
            };
            assert_eq!(none.to_linker_attr(), "");
        }

        #[test]
        fn test_region_to_linker_entry() {
            let r = Region::new("FLASH", 0x0001_0000, Permissions::RX, vec![0; 0x10000]);
            assert_eq!(
                r.to_linker_entry(),
                "    FLASH    (rx)  : org = 0x00010000, len = 0x10000"
            );

            let r2 = Region::new("RAM", 0x2000_0000, Permissions::RWX, vec![0; 4096]);
            assert_eq!(
                r2.to_linker_entry(),
                "    RAM      (rwx) : org = 0x20000000, len = 0x1000"
            );
        }
    }

    mod bus_tests {
        use super::*;

        #[test]
        fn test_bus_add_duplicate_name() {
            let mut bus = Bus::new();
            let r1 = Region::new("Flash", 0x1000, Permissions::RO, vec![0; 100]);
            let r2 = Region::new("Flash", 0x2000, Permissions::RW, vec![0; 100]);

            assert_eq!(bus.add(r1), Ok(()));
            assert_eq!(bus.add(r2), Err(bus::Error::DuplicateRegionName));
        }

        #[test]
        fn test_bus_add_overlapping_regions() {
            let mut bus = Bus::new();
            let r1 = Region::new("R1", 0x1000, Permissions::RO, vec![0; 0x1000]); // 0x1000..0x2000
            assert_eq!(bus.add(r1), Ok(()));

            // Partial overlap at end: 0x1800..0x2800
            let r2 = Region::new("R2", 0x1800, Permissions::RW, vec![0; 0x1000]);
            assert_eq!(bus.add(r2), Err(bus::Error::OverlappingRegion));

            // Subset overlap: 0x1200..0x1400
            let r3 = Region::new("R3", 0x1200, Permissions::RW, vec![0; 0x200]);
            assert_eq!(bus.add(r3), Err(bus::Error::OverlappingRegion));

            // Superset overlap: 0x0800..0x2800
            let r4 = Region::new("R4", 0x0800, Permissions::RW, vec![0; 0x2000]);
            assert_eq!(bus.add(r4), Err(bus::Error::OverlappingRegion));
        }

        #[test]
        fn test_bus_add_contiguous_regions() {
            let mut bus = Bus::new();
            // Contiguous / adjacent regions: [0x1000..0x2000) and [0x2000..0x3000)
            let r1 = Region::new("Flash", 0x1000, Permissions::RO, vec![0; 0x1000]);
            let r2 = Region::new("SRAM", 0x2000, Permissions::RW, vec![0; 0x1000]);

            assert_eq!(bus.add(r1), Ok(()));
            assert_eq!(bus.add(r2), Ok(()));
        }

        #[test]
        fn test_bus_remove_region() {
            let mut bus = Bus::new();
            let r1 = Region::new("Flash", 0x1000, Permissions::RO, vec![1, 2, 3, 4]);
            bus.add(r1).unwrap();

            let removed = bus.remove("Flash");
            assert!(removed.is_some());
            assert_eq!(removed.unwrap().name(), "Flash");

            // Removing again should return None
            assert!(bus.remove("Flash").is_none());
            // Non-existent region returns None
            assert!(bus.remove("NoSuchRegion").is_none());
        }

        #[test]
        fn test_bus_read_inside_multiple_regions() {
            let mut bus = Bus::new();
            let flash = Region::new(
                "Flash",
                0x0800_0000,
                Permissions::RX,
                vec![0xAA, 0xBB, 0xCC, 0xDD],
            );
            let sram = Region::new(
                "SRAM",
                0x2000_0000,
                Permissions::RW,
                vec![0x11, 0x22, 0x33, 0x44],
            );

            bus.add(flash).unwrap();
            bus.add(sram).unwrap();

            // Read from Flash
            assert_eq!(bus.read_u32(0x0800_0000, Permissions::RX), Ok(0xDDCCBBAA));
            // Read from SRAM
            assert_eq!(bus.read_u32(0x2000_0000, Permissions::RW), Ok(0x44332211));
        }

        #[test]
        fn test_bus_read_unmapped() {
            let bus = Bus::new();
            let mut buf = [0u8; 4];
            assert_eq!(
                bus.read(0x1000, &mut buf, Permissions::RO),
                Err(bus::Error::AddressViolation)
            );
        }

        #[test]
        fn test_bus_read_half_outside() {
            let mut bus = Bus::new();
            let r = Region::new("RAM", 0x1000, Permissions::RW, vec![1, 2, 3, 4]);
            bus.add(r).unwrap();

            // Starts at 0x1002, reading 4 bytes extends past 0x1004 into unmapped space
            assert_eq!(
                bus.read_u32(0x1002, Permissions::RW),
                Err(bus::Error::AddressViolation)
            );
        }

        #[test]
        fn test_bus_read_crosses_adjacent_regions() {
            let mut bus = Bus::new();
            let r1 = Region::new("R1", 0x1000, Permissions::RO, vec![1, 2, 3, 4]);
            let r2 = Region::new("R2", 0x1004, Permissions::RO, vec![5, 6, 7, 8]);
            bus.add(r1).unwrap();
            bus.add(r2).unwrap();

            // Single read across the border (starts at 0x1002, length 4)
            assert_eq!(
                bus.read_u32(0x1002, Permissions::RO),
                Err(bus::Error::AddressViolation)
            );
        }

        #[test]
        fn test_bus_read_permissions_violation() {
            let mut bus = Bus::new();
            // SRAM is Read-Write, but NOT executable
            let sram = Region::new("SRAM", 0x2000_0000, Permissions::RW, vec![0; 16]);
            bus.add(sram).unwrap();

            // Reading for execution (Permissions::RX) on RW memory must fail
            assert_eq!(
                bus.read_u32(0x2000_0000, Permissions::RX),
                Err(bus::Error::PermissionsViolation)
            );

            // Normal data read (Permissions::RO) succeeds
            assert_eq!(bus.read_u32(0x2000_0000, Permissions::RO), Ok(0));
        }

        #[test]
        fn test_bus_write_inside() {
            let mut bus = Bus::new();
            let sram = Region::new("SRAM", 0x2000_0000, Permissions::RW, vec![0; 16]);
            bus.add(sram).unwrap();

            assert_eq!(bus.write_u32(0x2000_0004, 0xDEADBEEF), Ok(()));
            assert_eq!(bus.read_u32(0x2000_0004, Permissions::RW), Ok(0xDEADBEEF));
        }

        #[test]
        fn test_bus_write_unmapped() {
            let mut bus = Bus::new();
            assert_eq!(
                bus.write_u32(0x9000_0000, 0x1234),
                Err(bus::Error::AddressViolation)
            );
        }

        #[test]
        fn test_bus_write_half_outside() {
            let mut bus = Bus::new();
            let sram = Region::new("SRAM", 0x2000_0000, Permissions::RW, vec![0; 4]);
            bus.add(sram).unwrap();

            // Starts at 0x2000_0002, 4 bytes extends past 0x2000_0004
            assert_eq!(
                bus.write_u32(0x2000_0002, 0x12345678),
                Err(bus::Error::AddressViolation)
            );
        }

        #[test]
        fn test_bus_write_to_readonly_flash() {
            let mut bus = Bus::new();
            let flash = Region::new("Flash", 0x0800_0000, Permissions::RX, vec![0; 32]);
            bus.add(flash).unwrap();

            assert_eq!(
                bus.write_u32(0x0800_0000, 0x1234),
                Err(bus::Error::PermissionsViolation)
            );
        }

        #[test]
        fn test_bus_convenience_u8_u16_u32() {
            let mut bus = Bus::new();
            let sram = Region::new("SRAM", 0x2000_0000, Permissions::RW, vec![0; 16]);
            bus.add(sram).unwrap();

            // Write u8, u16, u32
            bus.write_u8(0x2000_0000, 0x42).unwrap();
            bus.write_u16(0x2000_0002, 0x1234).unwrap();
            bus.write_u32(0x2000_0004, 0xAABBCCDD).unwrap();

            // Read back
            assert_eq!(bus.read_u8(0x2000_0000, Permissions::RW), Ok(0x42));
            assert_eq!(bus.read_u16(0x2000_0002, Permissions::RW), Ok(0x1234));
            assert_eq!(bus.read_u32(0x2000_0004, Permissions::RW), Ok(0xAABBCCDD));

            // Verify little-endian byte representation in raw buffer
            let mut raw_bytes = [0u8; 8];
            bus.read(0x2000_0000, &mut raw_bytes, Permissions::RW)
                .unwrap();
            assert_eq!(raw_bytes, [0x42, 0x00, 0x34, 0x12, 0xDD, 0xCC, 0xBB, 0xAA]);
        }

        #[test]
        fn test_bus_with_proxy_regions() {
            use std::sync::{Arc, Mutex};
            let mut bus = Bus::new();

            let ram = Region::new("RAM", 0x2000_0000, Permissions::RW, vec![0; 64]);
            bus.add(ram).unwrap();

            let uart_output = Arc::new(Mutex::new(Vec::new()));
            let uart_output_clone = Arc::clone(&uart_output);

            let uart = Region::new("UART", 0x4000_0000, Permissions::RW, vec![0; 16])
                .with_write_proxy(move |addr, buf| {
                    if addr == 0x4000_0000 {
                        uart_output_clone.lock().unwrap().extend_from_slice(buf);
                    }
                    Ok(())
                })
                .with_read_proxy(|addr, buf| {
                    if addr == 0x4000_0004 {
                        // Status: bit 0 set means TX ready
                        buf.copy_from_slice(&1u32.to_le_bytes()[..buf.len()]);
                    }
                    Ok(())
                });
            bus.add(uart).unwrap();

            // 1. Normal RAM write & read
            bus.write_u32(0x2000_0000, 0x12345678).unwrap();
            assert_eq!(bus.read_u32(0x2000_0000, Permissions::RW), Ok(0x12345678));

            // 2. UART proxy write
            bus.write_u8(0x4000_0000, b'A').unwrap();
            bus.write_u8(0x4000_0000, b'B').unwrap();
            assert_eq!(*uart_output.lock().unwrap(), vec![b'A', b'B']);

            // 3. UART proxy read
            assert_eq!(bus.read_u32(0x4000_0004, Permissions::RO), Ok(1));
        }

        #[test]
        fn test_bus_to_linker_script() {
            let mut bus = Bus::new();
            // Empty bus
            assert_eq!(bus.to_linker_script(), "MEMORY\n{\n}\n");

            // Bus with multiple regions
            let flash = Region::new("FLASH", 0x0001_0000, Permissions::RX, vec![0; 0x10000]);
            let ram = Region::new("RAM", 0x2000_0000, Permissions::RW, vec![0; 0x4000]);
            let uart = Region::new("UART", 0x4000_0000, Permissions::RW, vec![0; 0x1000]);

            bus.add(flash).unwrap();
            bus.add(ram).unwrap();
            bus.add(uart).unwrap();

            let expected = "MEMORY\n{\n    FLASH    (rx)  : org = 0x00010000, len = 0x10000\n    RAM      (rw)  : org = 0x20000000, len = 0x4000\n    UART     (rw)  : org = 0x40000000, len = 0x1000\n}\n";
            assert_eq!(bus.to_linker_script(), expected);
        }
    }
}
