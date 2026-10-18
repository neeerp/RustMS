#include "Guild.h"

#include <algorithm>
#include <cctype>
#include <utility>

namespace ms
{
	bool GuildEmblem::shown() const
	{
		return background != 0 || mark != 0;
	}

	bool GuildEmblem::operator==(const GuildEmblem& other) const
	{
		return background == other.background && background_color == other.background_color &&
			   mark == other.mark && mark_color == other.mark_color;
	}

	const char* guild_mark_category(uint16_t mark)
	{
		switch (mark / 1000)
		{
		case 2:
			return "Animal";
		case 3:
			return "Plant";
		case 4:
			return "Pattern";
		case 5:
			return "Letter";
		default:
			return "Etc";
		}
	}

	bool GuildMember::operator==(const GuildMember& other) const
	{
		return id == other.id && name == other.name && job == other.job && level == other.level &&
			   rank == other.rank && online == other.online;
	}

	bool GuildData::operator==(const GuildData& other) const
	{
		return id == other.id && name == other.name && titles == other.titles && members == other.members &&
			   capacity == other.capacity && emblem == other.emblem && notice == other.notice &&
			   points == other.points && alliance_id == other.alliance_id;
	}

	Guild::Name Guild::check_name(const std::string& name)
	{
		if (name.size() < MIN_NAME)
			return Name::TOO_SHORT;

		bool letters = std::all_of(name.begin(), name.end(), [](char c)
		{
			return std::isalnum(static_cast<unsigned char>(c)) != 0;
		});

		return name.size() <= MAX_NAME && letters ? Name::OK : Name::NOT_ALLOWED;
	}

	bool Guild::exists() const
	{
		return data.id != 0;
	}

	const GuildData& Guild::get_data() const
	{
		return data;
	}

	const GuildMember* Guild::find(int32_t cid) const
	{
		for (auto& member : data.members)
			if (member.id == cid)
				return &member;

		return nullptr;
	}

	const GuildMember* Guild::find_by_name(const std::string& name) const
	{
		if (!exists())
			return nullptr;

		for (auto& member : data.members)
			if (member.name == name)
				return &member;

		return nullptr;
	}

	bool Guild::is_master(int32_t cid) const
	{
		return exists() && !data.members.empty() && data.members.front().id == cid;
	}

	int32_t Guild::rank(int32_t cid) const
	{
		const GuildMember* member = find(cid);

		return member ? member->rank : 0;
	}

	bool Guild::full() const
	{
		return data.members.size() >= static_cast<size_t>(data.capacity);
	}

	bool Guild::may_kick(int32_t self, int32_t picked)
	{
		switch (picked)
		{
		case 2:
			return self == 1;
		case 3:
		case 4:
		case 5:
			return self == 1 || self == 2;
		default:
			return false;
		}
	}

	const std::string& Guild::title(int32_t rank) const
	{
		static const std::string none;

		if (rank < 1 || rank > static_cast<int32_t>(data.titles.size()))
			return none;

		return data.titles[rank - 1];
	}

	size_t Guild::online_count() const
	{
		return std::count_if(data.members.begin(), data.members.end(), [](const GuildMember& member)
		{
			return member.online;
		});
	}

	std::vector<int32_t> Guild::online_ids(int32_t self) const
	{
		std::vector<int32_t> result;

		for (auto& member : data.members)
			if (member.online && member.id != self)
				result.push_back(member.id);

		return result;
	}

	void Guild::update(GuildData new_data)
	{
		data = std::move(new_data);
	}

	void Guild::clear()
	{
		data = GuildData();
	}

	void Guild::add(GuildMember member)
	{
		remove(member.id);
		data.members.push_back(std::move(member));
	}

	void Guild::remove(int32_t cid)
	{
		auto& members = data.members;
		members.erase(std::remove_if(members.begin(), members.end(), [cid](const GuildMember& member)
		{
			return member.id == cid;
		}), members.end());
	}
}
