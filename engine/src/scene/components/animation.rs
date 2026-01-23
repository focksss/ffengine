use std::time::SystemTime;
use crate::math::Vector;
use crate::scene::scene::Entity;
use crate::scene::components::transform::Transform;

pub struct AnimationComponent {
    pub(crate) owner_entity: usize,
    pub channels: Vec<(usize, usize, String)>, // sampler index, impacted node, target transform component
    pub samplers: Vec<(Vec<f32>, String, Vec<Vector>)>, // input times, interpolation method, output vectors
    pub start_time: SystemTime,
    pub duration: f32,
    pub running: bool,
    pub repeat: bool,
    pub snap_back: bool,
}
impl AnimationComponent {
    pub fn start(&mut self) {
        self.start_time = SystemTime::now();
        self.running = true;
    }

    pub fn stop(&mut self, entities: &mut Vec<Entity>) {
        self.running = false;
        if self.snap_back {
            for channel in self.channels.iter() {
                entities[channel.1].animated_transform.1 = false;
            }
        }
    }

    pub fn update(&mut self, entities: &mut Vec<Entity>, transforms: &mut Vec<Transform>, unnupdated_entities: &mut Vec<usize>) {
        if !self.running {
            return
        }
        unnupdated_entities.push(self.owner_entity);
        let current_time = SystemTime::now();
        let elapsed_time = current_time.duration_since(self.start_time).unwrap().as_secs_f32();
        let mut repeat = false;
        if elapsed_time > self.duration {
            if self.repeat {
                repeat = true
            } else {
                self.stop(entities);
                return
            }
        }
        for channel in self.channels.iter() {
            let sampler = &self.samplers[channel.0];
            let mut current_time_index = 0;
            for i in 0..sampler.0.len() - 1 {
                if elapsed_time >= sampler.0[i] && elapsed_time < sampler.0[i + 1] {
                    current_time_index = i;
                    break
                }
            }
            let current_time_index = current_time_index.min(sampler.0.len() - 1);
            let interpolation_factor = ((elapsed_time - sampler.0[current_time_index]) / (sampler.0[current_time_index + 1] - sampler.0[current_time_index])).min(1.0).max(0.0);
            let vector1 = &sampler.2[current_time_index];
            let vector2 = &sampler.2[current_time_index + 1];
            let new_vector;
            if channel.2.eq("translation") || channel.2.eq("scale") {
                new_vector = Vector::new3(
                    vector1.x + interpolation_factor * (vector2.x - vector1.x),
                    vector1.y + interpolation_factor * (vector2.y - vector1.y),
                    vector1.z + interpolation_factor * (vector2.z - vector1.z),
                )
            } else {
                new_vector = Vector::spherical_lerp(vector1, vector2, interpolation_factor)
            }

            let entity = &mut entities[channel.1];
            entity.animated_transform.1 = true;
            let animated_transform = &mut transforms[entity.animated_transform.0];

            if channel.2.eq("translation") {
                animated_transform.local_translation = new_vector
            } else if channel.2.eq("rotation") {
                animated_transform.local_rotation = new_vector
            } else if channel.2.eq("scale") {
                animated_transform.local_scale = new_vector
            } else {
                panic!("Illogical animation channel target! Should be translation, rotation or scale");
            }
        }
        if repeat {
            self.start()
        }
    }
}