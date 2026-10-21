#include "Alliance.h"

#include <algorithm>
#include <utility>

namespace ms
{
	bool AllianceData::operator==(const AllianceData& other) const
	{
		return id == other.id && name == other.name && titles == other.titles && guild_ids == other.guild_ids &&
			   capacity == other.capacity && notice == other.notice;
	}

	bool Alliance::exists() const
	{
		return data.id != 0;
	}

	const AllianceData& Alliance::get_data() const
	{
		return data;
	}

	const std::vector<GuildData>& Alliance::get_guilds() const
	{
		return guilds;
	}

	const GuildData* Alliance::find_guild(int32_t id) const
	{
		for (auto& guild : guilds)
			if (guild.id == id)
				return &guild;

		return nullptr;
	}

	const std::string& Alliance::title(int32_t rank) const
	{
		static const std::string none;

		if (!exists() || rank < 1 || rank > static_cast<int32_t>(data.titles.size()))
			return none;

		return data.titles[rank - 1];
	}

	int32_t Alliance::lowest_rank() const
	{
		if (!exists())
			return 0;

		if (data.titles[3].empty())
			return 3;

		return data.titles[4].empty() ? 4 : 5;
	}

	bool Alliance::rank_held(const Guild& own, int32_t rank) const
	{
		if (!own.exists() || !exists())
			return true;

		auto holds = [rank](const GuildMember& member) { return member.alliance_rank == rank; };
		auto& members = own.get_data().members;

		if (std::any_of(members.begin(), members.end(), holds))
			return true;

		return std::any_of(guilds.begin(), guilds.end(), [&holds](const GuildData& guild)
		{
			return std::any_of(guild.members.begin(), guild.members.end(), holds);
		});
	}

	bool Alliance::joined(const Guild& own) const
	{
		return exists() && own.exists() && own.get_data().alliance_id != 0;
	}

	const GuildMember* Alliance::find_member(const Guild& own, int32_t cid) const
	{
		if (const GuildMember* member = own.find(cid))
			return member;

		for (auto& guild : guilds)
			for (auto& member : guild.members)
				if (member.id == cid)
					return &member;

		return nullptr;
	}

	const GuildData* Alliance::guild_of(const Guild& own, int32_t cid) const
	{
		if (own.find(cid))
			return &own.get_data();

		for (auto& guild : guilds)
			for (auto& member : guild.members)
				if (member.id == cid)
					return &guild;

		return nullptr;
	}

	bool Alliance::may_edit_titles(const Guild& own, int32_t self) const
	{
		const GuildMember* member = own.find(self);

		return joined(own) && member && member->alliance_rank == 1;
	}

	Guild::Title Alliance::check_title(const Guild& own, const std::array<std::string, 5>& working, int32_t rank,
									   const std::string& title) const
	{
		return Guild::check_title(working, rank, title, rank_held(own, rank));
	}

	bool Alliance::titles_differ(const std::array<std::string, 5>& working) const
	{
		return working != data.titles;
	}

	Alliance::TabButtons Alliance::tab_buttons(const Guild& own, int32_t self, int32_t picked, bool party_leader) const
	{
		TabButtons result;

		// Without an alliance, none.
		if (!joined(own))
			return result;

		const GuildMember* me = own.find(self);
		int32_t rank = me ? me->alliance_rank : -1;
		bool master = rank == 1;
		// Those needing someone else picked, whom the list shows online.
		const GuildMember* other = picked != self ? find_member(own, picked) : nullptr;

		if (other && !other->online)
			other = nullptr;

		int32_t lowest = lowest_rank();

		result.invite = master && data.guild_ids.size() < static_cast<size_t>(std::max(data.capacity, 0));
		result.withdraw = rank == 2;
		// Only another guild's master: their guild goes, or takes over.
		result.kick = master && other && other->rank == 1;
		result.change = result.kick;

		// The master or a guild master moves those of rank 3 and below. As
		// the client reads, rank 5 may always go down; the server refuses.
		if (other && (rank == 1 || rank == 2))
		{
			int32_t theirs = other->alliance_rank;
			result.up = theirs == 4 || theirs == 5;
			result.down = (theirs == 3 || theirs == 4) ? lowest != theirs : theirs == 5;
		}

		result.chat = other;
		result.whisper = other;
		result.info = true;
		result.notice = rank == 1 || rank == 2;
		result.party_invite = other && party_leader;

		return result;
	}

	std::vector<int32_t> Alliance::online_ids(const Guild& own, int32_t self) const
	{
		std::vector<int32_t> result;

		if (own.get_data().alliance_id == 0)
			return result;

		for (auto& guild : guilds)
			for (auto& member : guild.members)
				if (member.online)
					result.push_back(member.id);

		for (auto& member : own.get_data().members)
			if (member.online && member.id != self && member.id != 0)
				result.push_back(member.id);

		return result;
	}

	void Alliance::set(AllianceData new_data)
	{
		data = std::move(new_data);
	}

	void Alliance::clear()
	{
		data = AllianceData();
	}

	void Alliance::clear_guilds()
	{
		guilds.clear();
	}

	void Alliance::store(Guild& own, GuildData guild)
	{
		if (own.exists() && guild.id == own.get_data().id)
		{
			erase(guild.id);
			own.update(std::move(guild));
			return;
		}

		for (auto& kept : guilds)
		{
			if (kept.id == guild.id)
			{
				kept = std::move(guild);
				return;
			}
		}

		guilds.push_back(std::move(guild));
	}

	void Alliance::erase(int32_t guild_id)
	{
		guilds.erase(std::remove_if(guilds.begin(), guilds.end(), [guild_id](const GuildData& guild)
		{
			return guild.id == guild_id;
		}), guilds.end());
	}

	void Alliance::follow(const Guild& own)
	{
		if (own.get_data().alliance_id == 0)
		{
			clear();
			clear_guilds();
		}
		else
		{
			erase(own.get_data().id);
		}
	}

	const GuildMember* Alliance::set_online(int32_t guild_id, int32_t cid, bool online)
	{
		GuildMember* found = member(guild_id, cid);

		if (found)
			found->online = online;

		return found;
	}

	bool Alliance::set_level_job(int32_t guild_id, int32_t cid, int32_t level, int32_t job)
	{
		GuildMember* found = member(guild_id, cid);

		if (found)
		{
			found->level = level;
			found->job = job;
		}

		return found;
	}

	void Alliance::set_master(Guild& own, int32_t old_cid, int32_t new_cid)
	{
		own.set_alliance_rank(old_cid, 2);
		own.set_alliance_rank(new_cid, 1);

		if (GuildMember* old_master = member(old_cid))
			old_master->alliance_rank = 2;

		if (GuildMember* new_master = member(new_cid))
			new_master->alliance_rank = 1;
	}

	void Alliance::set_rank(Guild& own, int32_t cid, int32_t rank)
	{
		if (own.set_alliance_rank(cid, rank))
			return;

		if (GuildMember* found = member(cid))
			found->alliance_rank = rank;
	}

	void Alliance::set_titles(std::array<std::string, 5> titles)
	{
		data.titles = std::move(titles);
	}

	void Alliance::set_notice(std::string notice)
	{
		data.notice = std::move(notice);
	}

	GuildMember* Alliance::member(int32_t guild_id, int32_t cid)
	{
		for (auto& guild : guilds)
			if (guild.id == guild_id)
				for (auto& member : guild.members)
					if (member.id == cid)
						return &member;

		return nullptr;
	}

	GuildMember* Alliance::member(int32_t cid)
	{
		for (auto& guild : guilds)
			for (auto& member : guild.members)
				if (member.id == cid)
					return &member;

		return nullptr;
	}
}
