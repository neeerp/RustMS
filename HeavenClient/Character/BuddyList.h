#pragma once

#include <cstdint>
#include <set>
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
		// v83's GW_Friend flag: 1 while they ask to be added and the player
		// hasn't answered, 2 for someone always shown offline.
		uint8_t flag = 0;
		bool in_cash_shop = false;

		// A request, which v83 keeps in the list but never shows or counts.
		bool pending() const;
		// v83's online test (FUN_00908fc4): flags 0, 3 and 4 go by the
		// channel, 2 is always offline, and the rest always online.
		bool online() const;
		// Whether buddy chat goes to them (FUN_00a3fb25).
		bool hears_chat() const;
		bool operator==(const Buddy& other) const;
	};

	// The player's buddy list, as the server last described it.
	class BuddyList
	{
	public:
		static constexpr uint8_t DEFAULT_CAPACITY = 20;

		// What a channel update tells the player (FUN_00a3f2e8, mode 0x14).
		enum class Arrival { NONE, LOGGED_IN, SAME_CHANNEL };

		// Buddies in the server's order (by name), requests included.
		const std::vector<Buddy>& get_buddies() const;
		const Buddy* find(int32_t cid) const;
		// A buddy who isn't a request, by name regardless of case
		// (FUN_00a3fa90).
		const Buddy* find_listed(const std::string& name) const;
		// How many buddies there are, not counting requests.
		size_t listed() const;
		bool is_full() const;
		uint8_t get_capacity() const;

		// Buddies who would hear buddy chat, leaving out those blocked
		// (FUN_00a3fb52).
		std::vector<int32_t> online_ids(const std::set<int32_t>& blocked = {}) const;
		// Those of them in group (FUN_00a3fbc6).
		std::vector<int32_t> online_ids(const std::string& group, const std::set<int32_t>& blocked = {}) const;

		void update(std::vector<Buddy> buddies);
		// A request's entry, which v83 adds as it comes (FUN_00a40028).
		void add(Buddy buddy);
		// Overwrites the entry for cid, if there is one (FUN_00a400ad).
		void replace(int32_t cid, Buddy buddy);
		// own_channel: the player's, zero-based.
		Arrival set_channel(int32_t cid, int32_t channel, bool in_cash_shop, int32_t own_channel);
		void set_capacity(uint8_t capacity);

	private:
		std::vector<Buddy> buddies;
		uint8_t capacity = DEFAULT_CAPACITY;
	};
}
