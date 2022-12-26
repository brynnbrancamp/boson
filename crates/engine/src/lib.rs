#![feature(async_fn_in_trait)]
#![feature(default_free_fn)]
#![feature(box_syntax)]
#![feature(ptr_metadata)]

mod ecs;
mod utils;

//use crate::render::*;
use math::prelude::*;
use std::fs;
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub struct TestDrop {
    payload: String,
}

impl Drop for TestDrop {
    fn drop(&mut self) {
        log("testing drop");
        log(&self.payload);
    }
}

#[wasm_bindgen]
pub async fn start() -> Runtime {
    utils::set_panic_hook();

    use crate::ecs::*;

    let s = "gabe rundlett is the best".to_owned();

    let mut world = World::new();

    let e = world.spawn();

    let e2 = world.spawn();

    world.add(e, 32usize);
    world.add(
        e,
        TestDrop {
            payload: "payload".to_owned(),
        },
    );

    world.add(e2, 64usize);
    world.add(
        e2,
        TestDrop {
            payload: "payload2".to_owned(),
        },
    );

    let p = world.remove::<usize>(e);

    world.despawn(e);

    let mut schedule = Schedule::new();

    fn my_system(query: Query<(Entity, &String)>) {

    }

    schedule.add_stage(Stage::parallel()
        .add_system(my_system));

    /*let renderer = render::compatible().await;

    let vertices = [
        Vector::new([0.0, 0.0, 0.0, 1.0]),
        Vector::new([0.0, 1.0, 0.0, 1.0]),
        Vector::new([1.0, 1.0, 0.0, 1.0]),
    ]
    .into_iter()
    .map(|position| render::Vertex { position })
    .collect::<Vec<_>>();

    let indices = [0, 1, 2];

    let scene = renderer.scene("primary");

    scene.model("triangle")
        .vertices(&vertices)
        .indices(&indices);

    Runtime { renderer }*/
    Runtime {}
}

pub fn dilog<T: std::fmt::Display + ?Sized>(data: &T) {
    log(&format!("{data}"));
}

pub fn delog<T: std::fmt::Debug + ?Sized>(data: &T) {
    log(&format!("{data:?}"));
}

#[wasm_bindgen]
pub struct Runtime {
    //renderer: &'static dyn Renderer,
}

#[wasm_bindgen]
impl Runtime {
    pub async fn next(&mut self) {
        /*let render = Render {
            mvp: Matrix::identity(),
        };

        self.renderer.render(render, "primary");
        */
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
