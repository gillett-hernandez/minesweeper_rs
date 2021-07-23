# Minesweeper solver

## TODO

[x] isolate islands of Unknown blocks in the combinatorical simulation/consideration as independent from each other. should allow much higher limits for the number of islands that can be considered. will not improve the processing of larger islands however.
[ ] optimize combinatoric checking of unknown cells by restricting checked cells to be those that actually have empty cells around them.
[ ] optimize consistency checking of hypothetical gameboards by generating a list of empty cells to verify from the list of unknown cells that were changed.
