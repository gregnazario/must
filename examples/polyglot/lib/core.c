#include "core.h"
#include "hash.h"
#include "compress.h"

int32_t core_process(const uint8_t *input, size_t input_len,
                     uint8_t *output, size_t *output_len) {
    uint64_t h = core_hash(input, input_len);
    (void)h;
    return core_compress(input, input_len, output, output_len);
}

uint64_t core_hash(const uint8_t *data, size_t len) {
    return hash_fnv1a(data, len);
}

int32_t core_compress(const uint8_t *input, size_t input_len,
                      uint8_t *output, size_t *output_len) {
    return compress_zlib(input, input_len, output, output_len);
}
