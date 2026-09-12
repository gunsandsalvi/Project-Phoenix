/**
 * The kinds of capital this world has — and the TABLE IS THE KERNEL'S (13c, Law 4).
 *
 * @spec Capital Programme A1 Capital Programme A4 Capital Programme A5 Capital Programme A6 Capital Programme C3 Goods A2 Law 2 Law 15
 *
 * It was here as well as in `registry/physical.ts`, and two copies of one table disagree the moment
 * a kind gains a field — which is what happened when one did. Two modules need the list: this one
 * owns what a kind of plant DOES, and `firms` has to know which kinds a line needs to decide
 * anything at all; a module never imports another (ARCHITECTURE 4.9b). So the DATA is the kernel's,
 * beside the physical lines it is made from and put to work on, and the MECHANISM is this module's.
 *
 * A4 is what the table exists for: capital is SPECIFIC, and specific IN KIND — capital of one kind
 * is not capital of another, so it is counted in its own unit, made from its own good, and worn out
 * on its own schedule. A use that needs several kinds is limited by the scarcest of them, which is
 * why a row there is a row and not a column in one basket. And A4.b is why it has a life in it at
 * all: the presence of a life is what makes a good a capital good.
 */
export * from '../../registry/physical.js';
