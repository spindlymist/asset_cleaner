#include <stdio.h>
#include <string.h>

size_t float_to_str(float f, char *buffer, size_t buffer_size) {
    if (buffer_size == 0) {
        return 0;
    }
    
    // When MMF converts to a string, the output looks approximately like %g.
    // However, the exact formatting of %g doesn't seem to be standardized and differs between implementations.
    // I have not determined what implementation the MMF stdrt uses or whether it is dynamically linked.
    // On my machine, if an exponent is present, it is always 3 digits with a lowercase e and an explicit sign.
    // So, we will adjust the output to look like that if we have enough room.
    
    int length = snprintf(buffer, buffer_size, "%g", f);
    if (length <= 0 || length >= buffer_size) {
        return 0;
    }
    
    // Find the start of the exponent if there is one
    int index_e = 0;
    for (int i = 0; i < length; i++) {
        if (buffer[i] == 'e') {
            index_e = i;
            break;
        }
        else if (buffer[i] == 'E') {
            buffer[i] = 'e';
            index_e = i;
            break;
        }
    }
    if (index_e == 0) return length;
    
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
