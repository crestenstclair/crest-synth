#include "plugin_adapter.h"
namespace mda_Dynamics {
#include "../../vendor/audio/mda/plugins/mdaDynamics.cpp"
}
CrestProcessor* make_mda_Dynamics(float rate, size_t frames) {
    return new CrestPlugin(new mda_Dynamics::mdaDynamics(reinterpret_cast<void*>(1)), rate, frames, false);
}
