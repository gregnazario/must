#ifndef COMPRESS_H
#define COMPRESS_H

#include <stddef.h>
#include <stdint.h>

int32_t compress_zlib(const uint8_t *input, size_t input_len,
                      uint8_t *output, size_t *output_len);

int32_t decompress_zlib(const uint8_t *input, size_t input_len,
                        uint8_t *output, size_t *output_len);

#endif
