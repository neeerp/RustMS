#include "BuddyList.h"

#include <utility>

namespace ms
{
	bool Buddy::online() const
	{
		return channel >= 0;
	}

	bool Buddy::operator==(const Buddy& other) const
	{
		return id == other.id && name == other.name && group == other.group && channel == other.channel;
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

	bool BuddyList::is_full() const
	{
		return buddies.size() >= capacity;
	}

	uint8_t BuddyList::get_capacity() const
	{
		return capacity;
	}

	std::vector<int32_t> BuddyList::online_ids() const
	{
		std::vector<int32_t> result;

		for (auto& buddy : buddies)
			if (buddy.online())
				result.push_back(buddy.id);

		return result;
	}

	void BuddyList::update(std::vector<Buddy> new_buddies)
	{
		buddies = std::move(new_buddies);
	}

	void BuddyList::set_channel(int32_t cid, int32_t channel)
	{
		for (auto& buddy : buddies)
			if (buddy.id == cid)
				buddy.channel = channel;
	}

	void BuddyList::set_capacity(uint8_t new_capacity)
	{
		capacity = new_capacity;
	}
}
