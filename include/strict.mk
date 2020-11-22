CXXFLAGS += -Wall -Wextra -Wmisleading-indentation -Wduplicated-cond -Wduplicated-branches -Wshadow -Wnon-virtual-dtor -Wold-style-cast -Wcast-align -Wunused -Woverloaded-virtual -Wpedantic -Wconversion -Wsign-conversion -Wnull-dereference -Wdouble-promotion -Wformat=2
CXXFLAGS += -std=c++17
	   

#-Wall \                    # Reasonable and standard
#-Wextra \                  # Warn if indentation implies blocks where blocks do not exist.
#-Wmisleading-indentation \ # Warn if if / else chain has duplicated conditions
#-Wduplicated-cond \        # Warn if if / else branches has duplicated conditions
#-Wduplicated-branches \    # warn the user if a variable declaration shadows one from a parent context
#-Wshadow \                 # warn the user if a class with virtual functions has a non-virtual destructor. This helps
#-Wnon-virtual-dtor \       # catch hard to track down memory errors
#-Wold-style-cast \         # warn for c-style casts
#-Wcast-align \             # warn for potential performance problem casts
#-Wunused \                 # warn on anything being unused
#-Woverloaded-virtual \     # warn if you overload (not override) a virtual function
#-Wpedantic \               # warn if non-standard C++ is used
#-Wconversion \             # warn on type conversions that may lose data
#-Wsign-conversion \        # warn on sign conversions
#-Wnull-dereference \       # warn if a null dereference is detected
#-Wdouble-promotion \       # warn if float is implicit promoted to double
#-Wformat=2 \               # warn on security issues around functions that format output (ie printf)
