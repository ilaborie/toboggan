1. Simplify SlideId:

- Remove the Arc wrapper and conditional compilation
- Use a single static ID_SEQ: AtomicU8 = AtomicU8::new(0);

2. Streamline Content Types:

- Merge Html and Md into a single Html type

3. Reduce Feature Complexity:

- Consider making alloc mandatory (most embedded systems have heap)
- Remove test-utils feature - use #[cfg(test)] instead

Extra refactoring to do:

- Register render does not need a `renderer` so `Renderer` can be removed
- Add the `id: SlideId` into `Slide`, derive `Default` for `SlideId`, but inside a talk, ensure that all slides have an unique id for `0` to `n` if there are n slides.

Keep tests and documentation up to date.
