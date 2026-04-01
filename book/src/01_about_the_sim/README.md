# About the Simulation

As previously mentioned, this project uses the Bevy game engine. While Rust is
a fast and very powerful language, it can be difficult to do many thing, such
as work with large mutable structures, or display optimised and interactive
graphics.

I wanted to work with Rust, which meant I needed to overcome sever challenges.
Managing large state can be difficult -- especially when working iteratively. I
did not want to be required to refactor large sections whenever I added a
feature or made a change. Additionally, I didn't want to worry about graphics
too much. This led me to consider using a game engine for this project.

The biggest reason for using Bevy, however, is due to its' powerful ECS.
Simulations are all about responding to state, and ECS makes adding features
significantly simpler.

I will not go into details here as to how the Bevy engine or ECS works.
