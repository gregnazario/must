#include "compress.h"
#include <zlib.h>

int32_t compress_zlib(const uint8_t *input, size_t input_len,
                      uint8_t *output, size_t *output_len) {
    uLongf dest_len = (uLongf)(*output_len);
    int ret = compress2(output, &dest_len, input, (uLong)input_len, Z_DEFAULT_COMPRESSION);
    *output_len = (size_t)dest_len;
    return ret == Z_OK ? 0 : -1;
}

int32_t decompress_zlib(const uint8_t *input, size_t input_len,
                        uint8_t *output, size_t *output_len) {
    uLongf dest_len = (uLongf)(*output_len);
    int ret = uncompress(output, &dest_len, input, (uLong)input_len);
    *output_len = (size_t)dest_len;
    return ret == Z_OK ? 0 : -1;
}
