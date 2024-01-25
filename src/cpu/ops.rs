use super::Flags;

pub fn load_u8(target: &mut u16, status: &mut Flags, value: u8) {
    *target = value as u16;

    status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
    status.set(Flags::ZERO, value == 0);
}

pub fn load_u16(target: &mut u16, status: &mut Flags, value: u16) {
    *target = value;

    status.set(Flags::NEGATIVE, (value >> 15) & 1 == 1);
    status.set(Flags::ZERO, value == 0);
}

pub fn compare_u8(status: &mut Flags, lhs: u8, rhs: u8) {
    let result = lhs.wrapping_sub(rhs);

    status.set(Flags::NEGATIVE, (result >> 7) & 1 == 1);
    status.set(Flags::ZERO, result == 0);
    status.set(Flags::CARRY, lhs >= rhs);
}

pub fn compare_u16(status: &mut Flags, lhs: u16, rhs: u16) {
    let result = lhs.wrapping_sub(rhs);

    status.set(Flags::NEGATIVE, (result >> 15) & 1 == 1);
    status.set(Flags::ZERO, result == 0);
    status.set(Flags::CARRY, lhs >= rhs);
}

pub fn branch(pc: &mut u16, offset: u8, should_branch: bool) {
    if should_branch {
        let sign_bit = offset >> 7;

        // TODO: Is this overflow behaviour right, or should it increment the bank?
        if sign_bit == 1 {
            *pc = pc.wrapping_sub((!offset + 1) as u16);
        } else {
            *pc = pc.wrapping_add(offset as u16);
        }
    }
}

pub fn adc_u8(target: &mut u16, status: &mut Flags, value: u8) {
    let result = (*target as u8)
        .wrapping_add(value)
        .wrapping_add(status.contains(Flags::CARRY) as u8);

    status.set(Flags::NEGATIVE, (result >> 7) & 1 == 1);
    status.set(Flags::ZERO, result == 0);
    status.set(Flags::CARRY, result < *target as u8);

    // TODO: Overflow flag

    *target = result as u16;
}

pub fn adc_u16(target: &mut u16, status: &mut Flags, value: u16) {
    let result = (*target)
        .wrapping_add(value)
        .wrapping_add(status.contains(Flags::CARRY) as u16);

    status.set(Flags::NEGATIVE, (result >> 7) & 1 == 1);
    status.set(Flags::ZERO, result == 0);
    status.set(Flags::CARRY, result < *target);

    // TODO: Overflow flag

    *target = result;
}
