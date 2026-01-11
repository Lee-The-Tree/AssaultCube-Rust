use crate::{offset, util};

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn distance(&self, other: &Vec3) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }
}

pub struct Entity {
    pub process_handle: isize,
    pub base_addr: usize,
}

impl Entity {
    pub fn health(&self) -> i32 {
        util::read_memory::<i32>(self.process_handle, self.base_addr + offset::ENTITY_HEALTH as usize)
    }

    pub fn team(&self) -> i32 {
        util::read_memory::<i32>(self.process_handle, self.base_addr + offset::ENTITY_TEAM as usize)
    }

    pub fn name(&self) -> String {
        util::read_string(self.process_handle, self.base_addr + offset::ENTITY_NAME as usize, 16)
    }

    pub fn head_position(&self) -> Vec3 {
        let xyz = util::read_memory::<[f32; 3]>(self.process_handle, self.base_addr + offset::ENTITY_HEAD_POSITION as usize);
        Vec3 {
            x: xyz[0],
            y: xyz[1],
            z: xyz[2],
        }
    }

    pub fn feet_position(&self) -> Vec3 {
        let xyz = util::read_memory::<[f32; 3]>(self.process_handle, self.base_addr + offset::ENTITY_FEET_POSITION as usize);
        Vec3 {
            x: xyz[0],
            y: xyz[1],
            z: xyz[2],
        }
    }
}
