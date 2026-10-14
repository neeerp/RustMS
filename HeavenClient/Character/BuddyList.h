#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	struct Buddy
	{
		int32_t id = 0;
		std::string name;
		std::string group;
		// Zero-based channel, or negative while offline or until they add
		// the player back.
		int32_t channel = -1;

		bool online() const;
		bool operator==(const Buddy& other) const;
	};

	// The player's buddy list, as the server last described it.
	class BuddyList
	{
	public:
		static constexpr uint8_t DEFAULT_CAPACITY = 20;

		// Buddies in the server's order (by name).
		const std::vector<Buddy>& get_buddies() const;
		const Buddy* find(int32_t cid) const;
		bool is_full() const;
		uint8_t get_capacity() const;

		// Buddies who would hear buddy chat.
		std::vector<int32_t> online_ids() const;

		void update(std::vector<Buddy> buddies);
		void set_channel(int32_t cid, int32_t channel);
		void set_capacity(uint8_t capacity);

	private:
		std::vector<Buddy> buddies;
		uint8_t capacity = DEFAULT_CAPACITY;
	};
}
