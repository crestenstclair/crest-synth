#pragma once
#include "processor.h"
#include "CDSPResampler.h"
#include <algorithm>
#include <cmath>

// Streaming, preallocated rate/block adapter around the upstream r8brain
// resampler. A constant prepared delay absorbs converter and block batching.
class RateAdapter final : public CrestProcessor {
    std::unique_ptr<CrestProcessor> source_;
    r8b::CDSPResampler24 input_l_, input_r_, output_l_, output_r_;
    std::vector<double> input_left_, input_right_, output_left_, output_right_;
    std::vector<float> native_left_, native_right_, queued_left_, queued_right_;
    size_t native_count_ = 0, read_ = 0, write_ = 0, queued_ = 0, latency_;
public:
    RateAdapter(CrestProcessor* source, double source_rate, double host_rate, size_t block, size_t frames):
        source_(source), input_l_(host_rate,source_rate,frames), input_r_(host_rate,source_rate,frames),
        output_l_(source_rate,host_rate,block), output_r_(source_rate,host_rate,block),
        input_left_(frames), input_right_(frames), output_left_(block), output_right_(block),
        native_left_(block), native_right_(block) {
        latency_ = input_l_.getInputRequiredForOutput(output_l_.getInputRequiredForOutput(frames)+block)+frames;
        const size_t capacity = latency_ + frames + output_l_.getMaxOutLen(block) + 1;
        queued_left_.resize(capacity); queued_right_.resize(capacity);
        queued_ = latency_; write_ = latency_;
    }
    size_t count() const override { return source_->count(); }
    const char* label(size_t i) const override { return source_->label(i); }
    float initial(size_t i) const override { return source_->initial(i); }
    float minimum(size_t i) const override { return source_->minimum(i); }
    float maximum(size_t i) const override { return source_->maximum(i); }
    bool stepped(size_t i) const override { return source_->stepped(i); }
    size_t latency() const override { return latency_; }
    void set(size_t i, float v) override { source_->set(i,v); }
    void note(int status, int a, int b) override { source_->note(status,a,b); }
    bool load(const uint8_t* bytes,size_t size) override { return source_->load(bytes,size); }
    void reset() override {
        source_->reset(); input_l_.clear(); input_r_.clear(); output_l_.clear(); output_r_.clear();
        std::fill(queued_left_.begin(),queued_left_.end(),0);
        std::fill(queued_right_.begin(),queued_right_.end(),0);
        failed_=false; read_=native_count_=0; write_=queued_=latency_;
    }
    void process(float* left, float* right, size_t frames) override {
        for (size_t i=0; i<frames; ++i) { input_left_[i]=left[i]; input_right_[i]=right[i]; }
        double *l, *r;
        const int n=input_l_.process(input_left_.data(),frames,l);
        input_r_.process(input_right_.data(),frames,r);
        for (int i=0; i<n; ++i) {
            native_left_[native_count_]=l[i]; native_right_[native_count_]=r[i];
            if (++native_count_ != native_left_.size()) continue;
            source_->process(native_left_.data(),native_right_.data(),native_count_);
            for (size_t j=0; j<native_count_; ++j) { output_left_[j]=native_left_[j]; output_right_[j]=native_right_[j]; }
            double *ol,*oright;
            const int count=output_l_.process(output_left_.data(),native_count_,ol);
            output_r_.process(output_right_.data(),native_count_,oright);
            for (int j=0; j<count; ++j) {
                if (queued_ == queued_left_.size()) { failed_=true; return; }
                queued_left_[write_]=ol[j]; queued_right_[write_]=oright[j];
                write_=(write_+1)%queued_left_.size(); ++queued_;
            }
            native_count_=0;
        }
        for (size_t i=0; i<frames; ++i) {
            if (!queued_) { failed_=true; return; }
            left[i]=queued_left_[read_]; right[i]=queued_right_[read_];
            read_=(read_+1)%queued_left_.size(); --queued_;
        }
    }
    bool healthy() const override { return !failed_ && source_->healthy(); }
private:
    bool failed_ = false;
};
