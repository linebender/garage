# RustWeek 2025 animations

This is the source code for the sparse strip animations in my RustWeek
talk, [Faster, easier 2D vector rendering] (video). The [slides] for that
talk are also available.

The animations are implemented in Rust code running on top of Vello.
The code is quite hacky and not cleaned up – there's more than a bit
of creative mess. The codebase is based on a previous set of animations
for the [stroke expansion paper], and there are vestiges of that.

The sparse strip algorithm is cut-n-pasted from the sparse strip section
of [Vello]. Generally there's an attempt to be visually accurate, but a
the adapted code is written for ease of editing rather than generality
or performance.

[Faster, easier 2D vector rendering]: https://www.youtube.com/watch?v=_sv8K190Zps
[slides]: https://docs.google.com/presentation/d/1f_vKBJMaD68ifBO2j83lBly9Zdk-2bsvj_DIHXxvcuk
[stroke expansion paper]: https://github.com/linebender/gpu-stroke-expansion-paper
