#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <ctype.h>
#include <errno.h>
#include <math.h>

// Mimics MMF's double to string conversion.
//
// Returns the length of the result. buffer may or may not be null terminated.
size_t mmf_double_to_str(double value, char *buffer, size_t buffer_size) {
    // Check for NaN and infinity
    const char *special_value = NULL;
    switch (fpclassify(value)) {
        case FP_INFINITE:
            if (signbit(value) == 0) {
                special_value = "1.#INF";
            }
            else {
                special_value = "-1.#INF";
            }
            break;
        case FP_NAN:
            special_value = "-1.#IND";
            break;
    }

    if (special_value != NULL) {
        size_t length = strlen(special_value);
        if (length > buffer_size) {
            return 0;
        }
        else {
            strncpy(buffer, special_value, length);
            return length;
        }
    }
    
    // When MMF converts to a string, the output looks approximately like %g.
    // However, the exact formatting of %g doesn't seem to be standardized and differs between implementations.
    // I have not determined what implementation the MMF stdrt uses or whether it is dynamically linked.
    // On my machine, if an exponent is present, it is always 3 digits with a lowercase e and an explicit sign.
    // So, we will adjust the output to look like that if we have enough room.
    
    int length = snprintf(buffer, buffer_size, "%g", value);
    if (length <= 0 || length >= buffer_size) {
        return 0;
    }
    
    // Find the start of the exponent if there is one
    int index_e = 0;
    for (int i = 0; i < length; i++) {
        if (buffer[i] == 'e' || buffer[i] == 'E') {
            index_e = i;
            break;
        }
    }
    if (index_e == 0) return length;
    buffer[index_e] = 'e'; // Normalize case
    
    // Look for a sign and, if absent, insert a +
    int index_sign = index_e + 1;
    if (buffer[index_sign] != '+' && buffer[index_sign] != '-') {
        // Make sure we have enough room to insert a sign
        if (length + 1 > buffer_size) return length;
        
        // Shift the rest of the exponent 1 byte to the right
        memmove(&buffer[index_sign + 1], &buffer[index_sign], length - index_sign);
        buffer[index_sign] = '+';
        
        length += 1;
    }
    
    // Count the number of digits in the exponent
    int index_exp = index_sign + 1;
    int n_digits = length - index_exp;
    
    if (n_digits < 3) {
        // There are not enough digits, so we need to insert zeroes to compensate.
        // First, make sure there's enough room to pad the exponent
        int padding = 3 - n_digits;
        if (length + padding > buffer_size) return length;
        
        // Shift the rest of the exponent to the right
        memmove(&buffer[index_exp + padding], &buffer[index_exp], length - index_exp);
        
        // Fill in the left-padding with zeroes
        memset(&buffer[index_exp], '0', padding);
        
        length += padding;
    }
    else if (n_digits > 3) {
        // There are too many digits, so we need to remove some zeroes.
        int excess = n_digits - 3;
        
        // Move the last 3 digits to the start of the exponent
        memmove(&buffer[index_exp], &buffer[index_exp + excess], 3);
        
        // Discard the rest
        length -= excess;
    }
    
    return length;
}

typedef struct {
    union {
        int32_t value_i;
        double value_d;
    };
    bool is_double;
} MmfNumber;

// Mimics how MMF parses a number
//
// MMF will parse a number as a double if it has a . or e/E, even if there's nothing after.
//     123. == 123.0
//     123e == 123.0
//
// A leading - sign is allowed, but not a leading + sign.
//     +123 == 0
//     -123 == -123
//
// A leading . is not allowed. However, it is allowed after a negative sign.
//     -.123 == -0.123
//      .123 == 0
MmfNumber mmf_parse_number(const char *s) {
    MmfNumber num = {
        .value_i = 0,
        .is_double = false,
    };
    
    // Trim leading whitespace
    while (isspace(*s)) {
        s++;
    }
    
    // Leading negative sign is allowed
    // Any other leading character, including . or +, results in 0
    // . is allowed after -, however
    int i = 0;
    if (isdigit(s[0])) {
    }
    else if (s[0] == '-') {
        i++;
    }
    else {
        return num;
    }
    
    // Scan the rest of the string to see if it has a dot or e
    for (; s[i] != 0; i++) {
        if (isdigit(s[i])) {
            continue;
        }
        else if (s[i] == '.') {
            num.is_double = true;
            break;
        }
        else if (s[i] == 'e' || s[i] == 'E') {
            num.is_double = true;
            break;
        }
        else {
            break;
        }
    }
    
    if (num.is_double) {
        num.value_d = atof(s);
    }
    else {
        errno = 0;
        long value = strtol(s, NULL, 10);
        
        if (
            errno == ERANGE
            || value <= INT32_MIN + 1
            || value >= INT32_MAX
        ) {
            // Out-of-range integers become doubles
            num.is_double = true;
            num.value_d = atof(s);
        }
        else {
            num.value_i = (int32_t) value;
        }
    }
    
    return num;
}

// Mimics how Ini++ parses a number
//
// Ini++ will parse the number as a float if there is a dot anywhere in the string,
// even if it's not in the parseable range:
//     123abc.456 == 123.0
//
// Conversely, a number with an exponent but no dot will be parsed as an integer:
//     123e3 == 123
//
// Ini++ will parse as a 32-bit float, but MMF will store the number as a 64-bit double.
// Thus, even numbers that could be represented as a double may lose precision or become
// infinity.
MmfNumber ini_parse_number(const char *s) {
    MmfNumber num = {
        .value_i = 0,
        .is_double = false
    };

    if (strchr(s, '.') == NULL) {
        num.value_i = atoi(s);
    }
    else {
        float value_f = strtof(s, NULL);
        num.value_d = (double) value_f;
        num.is_double = true;
    }
    
    return num;
}
