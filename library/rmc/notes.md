
  1 /// The maximum allocation size is determined by the number of bits that
  124 /// are left in the pointer of width \p pointer_width.
    1 ///
	  2 /// The allocation size cannot exceed the number represented by the (signed)
	    3 /// offset, otherwise it would not be possible to store a pointer into a
		  4 /// valid bit of memory. Therefore, the max allocation size is
		    5 /// 2^(offset_bits - 1), where the offset bits is the number of bits left in the
			  6 /// pointer after the object bits.
