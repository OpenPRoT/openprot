// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#ifndef OPENPROT_UTIL_CONSOLE_DBG_PRINT_H_
#define OPENPROT_UTIL_CONSOLE_DBG_PRINT_H_

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

/**
 * An intentionally pared-down implementation of `printf()` that writes
 * to UART0.
 *
 * This function only supports the format specifiers required by the
 * ROM:
 * - %c prints a single character.
 * - %C prints a 'FourCC' style uint32_t (ASCII bytes in little-endian order).
 * - %d prints a signed int in decimal.
 * - %u prints an unsigned int in decimal.
 * - %s prints a nul-terminated string.
 * - %p prints pointer in hexadecimal.
 * - %x prints an `unsigned int` in hexadecimal using lowercase characters.
 *
 * No modifiers are supported and the leading zeros in hexidecimal
 * values are always printed.
 *
 * Note: unfortunately `uint32_t` is not necessarily an alias for
 * `unsigned int`. An explicit cast is therefore necessary when printing
 * `uint32_t` values using the `%x` format specifier in order to satisfy
 * the `printf` format checker (`-Wformat`).
 *
 * @param format The format specifier.
 * @param ... The values to interpolate in the format.
 * @return The result of the operation.
 */
void dbg_printf(const char *format, ...) __attribute__((format(printf, 1, 2)));

/**
 * Hexdump a region of memory.
 *
 * @param data The memory to dump.
 * @param len The length of the region to dump.
 */
void dbg_hexdump(const void *data, size_t len);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // OPENPROT_UTIL_CONSOLE_DBG_PRINT_H_
