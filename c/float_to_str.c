#include <stdio.h>

void float_to_str(float f, char *buffer, size_t buffer_size) {
    int n_bytes_written = snprintf(buffer, buffer_size, "%g", f);
    
    // Bail if buffer is full
    if (buffer_size == 0 || n_bytes_written >= buffer_size - 1) {
        return;
    }
    
    // Add leading 0 if there is a 2-digit exponent
    if (n_bytes_written >= 4 && buffer[n_bytes_written - 4] == 'e') {
        size_t exponent_start = n_bytes_written - 2;
        for (size_t i = n_bytes_written + 1; i > exponent_start; i--) {
            buffer[i] = buffer[i - 1];
        }
        buffer[exponent_start] = '0';
    }
}
