pub struct Mmu {
    cartridge: Vec<u8>,
    ram: Vec<u8>,

    spc: [u8; 4],
}

impl Mmu {
    pub fn new(cartridge: Vec<u8>) -> Mmu {
        Mmu {
            cartridge,
            ram: vec![0; 128000],

            spc: [0xAA, 0xBB, 0x00, 0x00],
        }
    }

    pub fn read_u8(&self, addr: u32) -> u8 {
        // TODO: This is hardcoded to LoROM at the moment.

        let bank = (addr >> 16) as u8;
        let offset = (addr & 0x00_FFFF) as u16;

        match bank {
            0x00..=0x3F => {
                match offset {
                    // RAM
                    0x0000..=0x1FFF => self.ram[offset as usize],

                    // Unused
                    0x2000..=0x20FF => 0,

                    // PPU, APU, Hardware
                    0x2100..=0x213F => 0,

                    // APUIO
                    0x2140..=0x2143 => self.spc[offset as usize - 0x2140],

                    // PPU, APU, Hardware
                    0x2144..=0x21FF => 0,

                    // Unused
                    0x2200..=0x2FFF => 0,

                    // DSP, SuperFX, Hardware
                    0x3000..=0x3FFF => 0,

                    // Joypads
                    0x4000..=0x40FF => 0,

                    // Unused
                    0x4100..=0x41FF => 0,

                    // DMA, PPU2, Hardware
                    0x4200..=0x44FF => 0,

                    // Unused
                    0x4500..=0x5FFF => 0,

                    // Enhancement
                    0x6000..=0x7FFF => 0,

                    // ROM
                    0x8000..=0xFFFF => {
                        self.cartridge[(offset - 0x8000) as usize + (bank as usize * 0x8000)]
                    }
                }
            }

            0x7E => self.ram[offset as usize],

            _ => unimplemented!("bank {:02X} is unimplemented", bank),
        }
    }

    pub fn store_u8(&mut self, addr: u32, value: u8) {
        // TODO: This is hardcoded to LoROM at the moment.

        let bank = (addr >> 16) as u8;
        let offset = (addr & 0x00_FFFF) as u16;

        match bank {
            0x00..=0x3F => {
                match offset {
                    // RAM
                    0x0000..=0x1FFF => self.ram[offset as usize] = value,

                    // Unused
                    0x2000..=0x20FF => {}

                    // PPU, APU, Hardware
                    0x2100..=0x213F => {}

                    // APUIO
                    0x2140..=0x2143 => self.spc[offset as usize - 0x2140] = value,

                    // PPU, APU, Hardware
                    0x2144..=0x21FF => {}

                    // Unused
                    0x2200..=0x2FFF => {}

                    // DSP, SuperFX, Hardware
                    0x3000..=0x3FFF => {}

                    // Joypads
                    0x4000..=0x40FF => {}

                    // Unused
                    0x4100..=0x41FF => {}

                    // DMA, PPU2, Hardware
                    0x4200..=0x44FF => {}

                    // Unused
                    0x4500..=0x5FFF => {}

                    // Enhancement
                    0x6000..=0x7FFF => {}

                    // ROM
                    0x8000..=0xFFFF => {}
                }
            }

            0x7E => self.ram[offset as usize] = value,

            _ => {
                // TODO: Implement rest of memory ranges
            }
        }
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        let byte0 = self.read_u8(addr);
        let byte1 = self.read_u8(addr + 1);

        u16::from_le_bytes([byte0, byte1])
    }

    pub fn read_long(&self, addr: u32) -> u32 {
        let byte0 = self.read_u8(addr);
        let byte1 = self.read_u8(addr + 1);
        let byte2 = self.read_u8(addr + 2);

        u32::from_le_bytes([byte0, byte1, byte2, 0])
    }

    pub fn store_u16(&mut self, addr: u32, value: u16) {
        let [byte0, byte1] = value.to_le_bytes();

        self.store_u8(addr, byte0);
        self.store_u8(addr + 1, byte1)
    }

    pub fn reset_vector(&self) -> u16 {
        u16::from_le_bytes([self.cartridge[0x7FFC], self.cartridge[0x7FFD]])
    }
}
