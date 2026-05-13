#ifndef HASH_H
#define HASH_H

#include <stddef.h>
#include <stdint.h>

uint64_t hash_fnv1a(const uint8_t *data, size_t len);

#endif
