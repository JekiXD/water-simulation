## Water simulation using particles written with WebGPU
To start the program, run the following commands in the main directory:
```
cargo build --all
cargo run
```

If simulation is lagging, you can try  decreasing the amount of particles in `settings.rs` file. Or you can try decreasing the size of a grid used for neighbour search(`grid_size`). If you want to do the latter, then there are 2 options:

- You can change it with a menu after launching the program. In this case, if you decrease the size of the grid, you will see artifacts, mainly the grid itself. This happens due to radius of kernels being smaller than the grid. You can then changes kernel's sizes accordingly
- Or change it in `settings.rs` file to preserve kernel's sizes to be equal to grid size.

Beware, that changing kernel's radiuses is going to destabilize the system. You'll have to play with values(the main ones are "Rest density" and "Near/Pressure multiplier") to find a new balance.

