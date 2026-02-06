import { CHUNK_VOLUME } from './constants.js';

export class Chunk {
    constructor(cx, cz, blocks = null) {
        this.cx = cx;
        this.cz = cz;
        this.blocks = blocks ?? new Uint8Array(CHUNK_VOLUME);
        this.mesh = null;
        this.dirty = true;
        this.state = 'new';
    }
}
