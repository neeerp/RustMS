#include "Guild.h"

#include "../Util/CharacterName.h"

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

		return character_name_valid(name) ? Name::OK : Name::NOT_ALLOWED;
	}

	size_t Guild::min_title(int32_t rank)
	{
		return rank <= 3 ? MIN_NAME : 0;
	}

	Guild::Title Guild::check_title(const std::array<std::string, 5>& working, int32_t rank,
									const std::string& title) const
	{
		if (title.size() < min_title(rank))
			return Title::TOO_SHORT;

		if (!title.empty())
			return character_name_valid(title) ? Title::OK : Title::NOT_ALLOWED;

		if (rank == 4 && !working[4].empty())
			return Title::LOWER_RANK;

		return rank_held(rank) ? Title::HELD : Title::OK;
	}

	void Guild::set_title(std::array<std::string, 5>& working, int32_t rank, std::string title)
	{
		working[rank - 1] = std::move(title);

		if (rank == 5 && working[3].empty() && !working[4].empty())
			working[3] = "Member";
	}

	bool Guild::titles_differ(const std::array<std::string, 5>& working) const
	{
		for (size_t i = 0; i < working.size(); i++)
		{
			const std::string& stored = data.titles[i];
			bool same = std::equal(stored.begin(), stored.end(), working[i].begin(), working[i].end(),
								   [](char a, char b)
			{
				return std::tolower(static_cast<unsigned char>(a)) == std::tolower(static_cast<unsigned char>(b));
			});

			if (!same)
				return true;
		}

		return false;
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

	bool Guild::may_raise(int32_t self, int32_t picked)
	{
		switch (self)
		{
		case 1:
			return picked >= 3;
		case 2:
			return picked >= 4;
		default:
			return false;
		}
	}

	bool Guild::may_lower(int32_t self, int32_t picked) const
	{
		switch (self)
		{
		case 1:
			return picked >= 2 && picked != lowest_rank();
		case 2:
			return picked >= 3 && picked != lowest_rank();
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

	int32_t Guild::lowest_rank() const
	{
		if (!exists())
			return 0;

		if (data.titles[3].empty())
			return 3;

		return data.titles[4].empty() ? 4 : 5;
	}

	bool Guild::rank_held(int32_t rank) const
	{
		return !exists() || std::any_of(data.members.begin(), data.members.end(), [rank](const GuildMember& member)
		{
			return member.rank == rank;
		});
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

	bool Guild::set_rank(int32_t cid, int32_t rank)
	{
		GuildMember* found = member(cid);

		if (found)
			found->rank = rank;

		return found;
	}

	bool Guild::set_online(int32_t cid, bool online)
	{
		GuildMember* found = member(cid);

		if (found)
			found->online = online;

		return found;
	}

	bool Guild::set_level_job(int32_t cid, int32_t level, int32_t job)
	{
		GuildMember* found = member(cid);

		if (found)
		{
			found->level = level;
			found->job = job;
		}

		return found;
	}

	void Guild::set_titles(std::array<std::string, 5> titles)
	{
		data.titles = std::move(titles);
	}

	void Guild::set_notice(std::string notice)
	{
		data.notice = std::move(notice);
	}

	void Guild::set_capacity(int32_t capacity)
	{
		data.capacity = capacity;
	}

	void Guild::set_emblem(const GuildEmblem& emblem)
	{
		data.emblem = emblem;
	}

	GuildMember* Guild::member(int32_t cid)
	{
		for (auto& member : data.members)
			if (member.id == cid)
				return &member;

		return nullptr;
	}
}
