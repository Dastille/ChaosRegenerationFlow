extern "C" __global__ void chaos_gen(unsigned long long seed, unsigned short tweak, size_t len, unsigned char *output) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < len) {
        unsigned long long val = seed + idx;
        val = (val * 314159 + 299792458 + tweak) % 256;
        output[idx] = (unsigned char)val;
    }
}
