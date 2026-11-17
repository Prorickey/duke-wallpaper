use crate::canvas::Canvas;
use crate::environment::Environment;

#[derive(Clone, Debug, Default)]
pub struct SceneInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub lightning_strike_points: Vec<(i32, i32)>,
}

pub trait Scene {
    fn info(&self) -> SceneInfo;

    fn render(&mut self, environment: &Environment, canvas: &mut Canvas);

    fn render_lights(&mut self, environment: &Environment, canvas: &mut Canvas);
}
