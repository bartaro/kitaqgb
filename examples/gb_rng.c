// Generate a repeatable pseudorandom value from a seed.
// Expected: value 000 through 009; same seed repeats
#include "gb_common.h"
#include "rpg.h"

// Seed the generator with 1234 and display one value in [0,10).
// Restarting with the same seed reproduces the sequence.
void main() {
    m_init();
    m_text(2,3,"RNG");
    rng_seed(1234); m_number(rng_range(10));
    while (1) { m_wait();  }
}
