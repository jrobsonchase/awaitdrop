# Yet another WaitGroup implementation.

None of the existing crates fit my needs exactly, so here's one more that
(hopefully) will. 

Highlights:
* Generalizes "tasks" to [Ref][ref]s. More of a change in nomenclature than
  anything else. It's not always a group of tasks you're waiting on - it
  could be that you're waiting on a gaggle of structs to all be dropped.
* [Ref][ref]s and [Waiter][waiter]s are entirely disjoint. You don't need a
  [Waiter][waiter] to create a new [Ref][ref].
* Everything is cloneable and behaves as one would expect - cloned [Ref][ref]s
  will all block every cloned [Waiter][waiter], which can be awaited concurrently.

[ref]: https://docs.rs/awaitdrop/latest/awaitdrop/struct.Ref.html
[waiter]: https://docs.rs/awaitdrop/latest/awaitdrop/struct.Waiter.html

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in tokio-core by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.