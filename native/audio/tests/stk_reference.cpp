#define BandedWG ReferenceBandedWG
#define Mesh2D ReferenceMesh2D
#include "../../../vendor/audio/stk/include/BandedWG.h"
#include "../../../vendor/audio/stk/src/BandedWG.cpp"
#include "../../../vendor/audio/stk/src/Mesh2D.cpp"
#undef BandedWG
#undef Mesh2D
#include "stk_sequence.h"
std::vector<double> stk_reference_sequence(){return stk_model_sequence<stk::ReferenceBandedWG,stk::ReferenceMesh2D>();}
