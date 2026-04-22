// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#include "util/console/dbg_print.h"

#include <assert.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>

extern void system_lowlevel_console_write(const char *buf, size_t len);

static const char kHexTable[17] = "0123456789abcdef";

static size_t print_integer(char *dest, unsigned value, bool is_signed) {
    char buf[12];
    char *b = buf + sizeof(buf);
    size_t len = 0;
    if (is_signed && (int)value < 0) {
        *dest++ = ('-');
        len++;
        value = (unsigned)(-(int)value);
    }
    *--b = '\0';
    do {
        *--b = '0' + value % 10;
        value /= 10;
    } while (value);
    while (*b) {
        *dest++ = (*b++);
        len++;
    }
    return len;
}

void dbg_printf(const char *format, ...) {
    char buffer[256];
    char *buf = buffer;

    va_list args;
    va_start(args, format);

    for (; *format != '\0'; ++format) {
        if (*format != '%') {
            *buf++ = *format;
            continue;
        }

        ++format;  // Skip over the '%'.
        switch (*format) {
            case '%':
                *buf++ = *format;
                break;
            case 'c': {
                int ch = va_arg(args, int);
                *buf++ = (char)ch;
                break;
            }
            case 'C': {
                uint32_t val = va_arg(args, uint32_t);
                for (size_t i = 0; i < sizeof(uint32_t); ++i, val >>= 8) {
                    uint8_t ch = (uint8_t)val;
                    if (ch >= 32 && ch < 127) {
                        *buf++ = (char)ch;
                    } else {
                        *buf++ = ('\\');
                        *buf++ = ('x');
                        *buf++ = (kHexTable[ch >> 4]);
                        *buf++ = (kHexTable[ch & 15]);
                    }
                }
                break;
            }
            case 's': {
                // Print a null-terminated string.
                const char *str = va_arg(args, const char *);
                while (*str != '\0') {
                    *buf++ = (*str++);
                }
                break;
            }
            case 'd':
                // `print_integer` will handle the sign bit of the value.
                buf += print_integer(buf, va_arg(args, unsigned), true);
                break;
            case 'u':
                buf += print_integer(buf, va_arg(args, unsigned), false);
                break;
            case 'p':
            case 'x': {
                // Print an `unsigned int` in hexadecimal.
                unsigned int v = va_arg(args, unsigned int);
                for (size_t i = 0; i < sizeof(v) * 2; ++i) {
                    int shift = sizeof(v) * 8 - 4;
                    *buf++ = (kHexTable[v >> shift]);
                    v <<= 4;
                }
                break;
            }
            default:
                // For an invalid format specifier, back up one char and allow
                // the output via the normal mechanism.
                *buf++ = ('%');
                --format;
        }
    }
    va_end(args);
    system_lowlevel_console_write(buffer, buf - buffer);
}

void dbg_hexdump(const void *data, size_t len) {
    const uint8_t *p = (const uint8_t *)data;
    size_t j = 0;

    while (j < len) {
        // hexbuf is initialized as 48 spaces followed by a nul byte.
        char hexbuf[] = "                                                ";
        // ascii is initialized as 17 nul bytes.
        char ascii[17] = {
            0,
        };
        dbg_printf("%p: ", p);
        for (size_t i = 0; i < 16 && j < len; ++p, ++i, ++j) {
            uint8_t val = *p;
            hexbuf[i * 3 + 0] = kHexTable[val >> 4];
            hexbuf[i * 3 + 1] = kHexTable[val & 15];
            ascii[i] = (val >= 32 && val < 127) ? (char)val : '.';
        }
        dbg_printf("%s  %s\r\n", hexbuf, ascii);
    }
}
