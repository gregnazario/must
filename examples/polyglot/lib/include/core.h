#ifndef CORE_H
#define CORE_H

#include <stddef.h>
#include <stdint.h>

int32_t core_process(const uint8_t *input, size_t input_len,
                     uint8_t *output, size_t *output_len);

uint64_t core_hash(const uint8_t *data, size_t len);

int32_t core_compress(const uint8_t *input, size_t input_len,
                      uint8_t *output, size_t *output_len);

#endif
