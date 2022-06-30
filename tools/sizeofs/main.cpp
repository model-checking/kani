// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#include <iostream>
#include <type_traits>
 
 // generates the arch-dependent constants for MachineModel
int main() 
{    
    std::cout << "let bool_width = " << sizeof(bool)*8 << ";" << std::endl;
    std::cout << "let char_is_unsigned = " << (std::is_unsigned<bool>::value?"true":"false") << ";" << std::endl;
    std::cout << "let char_width = " << sizeof(char)*8 << ";" << std::endl;   
    std::cout << "let double_width = " << sizeof(double)*8 << ";" << std::endl;
    std::cout << "let float_width = " << sizeof(float)*8 << ";" << std::endl;
    std::cout << "let int_width = " << sizeof(int)*8 << ";" << std::endl;
    std::cout << "let long_double_width = " << sizeof(long double)*8 << ";" << std::endl;
    std::cout << "let long_int_width = " << sizeof(long int)*8 << ";" << std::endl;
    std::cout << "let long_long_int_width = " << sizeof(long long int)*8 << ";" << std::endl;
    // memory_operand_size
    // null_is_zero 
    std::cout << "let short_int_width = " << sizeof(short int)*8 << ";" << std::endl;
    std::cout << "let single_width = " << sizeof(float)*8 << ";" << std::endl;
    std::cout << "let wchar_t_is_unsigned = " << (std::is_unsigned<wchar_t>::value?"true":"false") << ";" << std::endl;
    std::cout << "let wchar_t_width = " << sizeof(wchar_t)*8 << ";" << std::endl;    
    // word_size
    // rounding_mode    
}

 