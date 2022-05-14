# revproc1

Simulation of a very simple reversible multithreaded processor, using an instruction set that I hope will one day be a superset of RISC-V

Memory is modelled explicitly via asynchronous packet I/O.
The discard stack is held in a (possibly differently addressed) piece of memory.
Input files are lists of instructions, executed after reset:

[
  { "code": "mov", "op1" : { "mode" : "reg", "val" : "r1" } } 
]



