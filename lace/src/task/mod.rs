#[cfg(not(feature = "unsafe_task"))]
pub mod safe;
#[cfg(not(feature = "unsafe_task"))]
pub use safe as picked;

#[cfg(feature = "unsafe_task")]
pub mod r#unsafe;
#[cfg(feature = "unsafe_task")]
pub use r#unsafe as picked;

#[cfg(test)]
mod tests {
    use super::picked::*;
    use crate::Worker;
    use std::hint::black_box;

    fn id_task(_: &mut Worker, x: usize) -> usize {
        x
    }

    #[test]
    fn execute() {
        let mut worker = Worker::mock();
        let task = Task::new(&mut worker.arena, id_task, 123);
        assert_eq!(task.execute(&mut worker), 123);
    }
    #[test]
    fn execute_stolen() {
        let mut worker = Worker::mock();
        let task = Task::new(&mut worker.arena, id_task, 123);
        let mut tet = task.erase();
        tet.execute_stolen(&mut worker);
        let mut task = tet.unerase::<usize, usize>();
        assert_eq!(task.take_output(&mut worker.arena), 123);
    }

    #[test]
    fn stress() {
        let mut worker = Worker::mock();
        for _ in 0..100000000 {
            let task = Task::new(&mut worker.arena, id_task, 140);
            // the black_box represents the task going in and out of the buffer
            let tet = black_box(task.erase());
            let task = tet.unerase::<usize, usize>();
            let x = task.execute(&mut worker);
            assert_eq!(x, 140);
        }
    }
}
