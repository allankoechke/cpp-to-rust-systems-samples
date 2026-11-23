// g++ -O2 -std=c++20 cache_lab.cpp -o cache_lab && ./cache_lab
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <vector>

int main() {
    const size_t N = size_t(1) << 26;        // 64M ints = 256 MiB, far bigger than any cache
    const size_t touches = size_t(1) << 22;  // same amount of work for every stride
    std::vector<int32_t> a(N, 1);
    int64_t sink = 0;
    for (size_t stride : {1, 2, 4, 8, 16, 32, 64, 256, 1024, 4096}) {
        auto t0 = std::chrono::steady_clock::now();
        for (size_t k = 0; k < touches; ++k) sink += a[(k * stride) & (N - 1)];
        auto t1 = std::chrono::steady_clock::now();
        double ns = std::chrono::duration<double, std::nano>(t1 - t0).count() / touches;
        std::printf("stride %5zu  %6.2f ns/element\n", stride, ns);
    }
    std::printf("(ignore) %lld\n", (long long)sink);  // keeps the loop from being optimised away
}
