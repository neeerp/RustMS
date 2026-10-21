#include "BuddyList.h"

#include <algorithm>
#include <cctype>
#include <utility>

namespace ms
{
	namespace
	{
		bool same_name(const std::string& a, const std::string& b)
		{
			return a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin(),
				[](unsigned char x, unsigned char y) { return std::tolower(x) == std::tolower(y); });
		}

		// The flags whose online state follows the channel.
		bool by_channel(uint8_t flag)
		{
			return flag == 0 || flag == 3 || flag == 4;
		}
	}

	bool Buddy::pending() const
	{
		return flag == 1;
	}

	bool Buddy::online() const
	{
		return flag != 2 && !(by_channel(flag) && channel < 0);
	}

	bool Buddy::hears_chat() const
	{
		return id != 0 && by_channel(flag) && channel >= 0;
	}

	bool Buddy::operator==(const Buddy& other) const
	{
		return id == other.id && name == other.name && group == other.group && channel == other.channel
			&& flag == other.flag && in_cash_shop == other.in_cash_shop;
	}

	const std::vector<Buddy>& BuddyList::get_buddies() const
	{
		return buddies;
	}

	const Buddy* BuddyList::find(int32_t cid) const
	{
		for (auto& buddy : buddies)
			if (buddy.id == cid)
				return &buddy;

		return nullptr;
	}

	const Buddy* BuddyList::find_listed(const std::string& name) const
	{
		for (auto& buddy : buddies)
			if (!buddy.pending() && same_name(buddy.name, name))
				return &buddy;

		return nullptr;
	}

	size_t BuddyList::listed() const
	{
		return std::count_if(buddies.begin(), buddies.end(), [](const Buddy& buddy) { return !buddy.pending(); });
	}

	bool BuddyList::is_full() const
	{
		return listed() >= capacity;
	}

	uint8_t BuddyList::get_capacity() const
	{
		return capacity;
	}

	std::vector<int32_t> BuddyList::online_ids(const std::set<int32_t>& blocked) const
	{
		std::vector<int32_t> result;

		for (auto& buddy : buddies)
			if (buddy.hears_chat() && !blocked.count(buddy.id))
				result.push_back(buddy.id);

		return result;
	}

	std::vector<int32_t> BuddyList::online_ids(const std::string& group, const std::set<int32_t>& blocked) const
	{
		std::vector<int32_t> result;

		for (auto& buddy : buddies)
			if (buddy.group == group && buddy.hears_chat() && !blocked.count(buddy.id))
				result.push_back(buddy.id);

		return result;
	}

	void BuddyList::update(std::vector<Buddy> new_buddies)
	{
		buddies = std::move(new_buddies);
	}

	void BuddyList::add(Buddy buddy)
	{
		buddies.push_back(std::move(buddy));
	}

	void BuddyList::replace(int32_t cid, Buddy buddy)
	{
		for (auto& old : buddies)
			if (old.id == cid)
			{
				old = std::move(buddy);
				return;
			}
	}

	// Coming online counts first; otherwise only a move onto the player's
	// channel is told.
	BuddyList::Arrival BuddyList::set_channel(int32_t cid, int32_t channel, bool in_cash_shop, int32_t own_channel)
	{
		for (auto& buddy : buddies)
		{
			if (buddy.id != cid)
				continue;

			int32_t old = buddy.channel;

			if (old == channel && buddy.in_cash_shop == in_cash_shop)
				return Arrival::NONE;

			buddy.channel = channel;
			buddy.in_cash_shop = in_cash_shop;

			if (old < 0 && channel >= 0)
				return Arrival::LOGGED_IN;

			if (old != channel && channel == own_channel)
				return Arrival::SAME_CHANNEL;

			return Arrival::NONE;
		}

		return Arrival::NONE;
	}

	void BuddyList::set_capacity(uint8_t new_capacity)
	{
		capacity = new_capacity;
	}
}
