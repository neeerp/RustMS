#include "../Template/TimedQueue.h"
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

struct Shot
{
	std::string star;
	int hit;

	Shot(std::string star, int hit) : star(std::move(star)), hit(hit) {}
};

int main()
{
	try
	{
		std::vector<Shot> landed;
		ms::TimedQueue<Shot> queue([&](const Shot& shot) { landed.push_back(shot); });

		// A shot of two stars queues the same star twice; the first mustn't
		// take it from the second.
		std::string star = "subi";
		queue.emplace(0, star, 1);
		queue.emplace(10, star, 2);
		require(star == "subi", "queueing a star leaves the caller's star alone");

		queue.update(5);
		require(landed.size() == 1 && landed[0].hit == 1, "the first star lands first");
		queue.update(10);
		require(landed.size() == 2 && landed[1].star == "subi", "the second star is still a star");

		std::cout << "Timed queue passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
