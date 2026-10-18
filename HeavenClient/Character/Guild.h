#pragma once

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// A guild's emblem: a mark over a background, each in one of 16 colours.
	struct GuildEmblem
	{
		uint16_t background = 0;
		uint8_t background_color = 0;
		uint16_t mark = 0;
		uint8_t mark_color = 0;

		// The client draws an emblem only if the background or the mark is
		// set.
		bool shown() const;
		bool operator==(const GuildEmblem& other) const;
	};

	// The GuildMark.img/Mark folder of a mark, by its thousands.
	const char* guild_mark_category(uint16_t mark);

	struct GuildMember
	{
		int32_t id = 0;
		std::string name;
		int32_t job = 0;
		int32_t level = 0;
		// 1 for the master, down to 5.
		int32_t rank = 0;
		bool online = false;

		bool operator==(const GuildMember& other) const;
	};

	// What GUILDDATA carries.
	struct GuildData
	{
		int32_t id = 0;
		std::string name;
		// The titles of ranks 1 to 5.
		std::array<std::string, 5> titles;
		// The master first.
		std::vector<GuildMember> members;
		int32_t capacity = 0;
		GuildEmblem emblem;
		std::string notice;
		int32_t points = 0;
		int32_t alliance_id = 0;

		bool operator==(const GuildData& other) const;
	};

	// The player's guild, as the server last described it.
	class Guild
	{
	public:
		// The guild name prompt's bounds.
		static constexpr size_t MIN_NAME = 4;
		static constexpr size_t MAX_NAME = 12;

		enum class Name
		{
			OK,
			TOO_SHORT,
			NOT_ALLOWED
		};

		// Checks a new guild's name as the client does, but for the
		// forbidden words: 4 to 12 letters or digits.
		static Name check_name(const std::string& name);

		bool exists() const;
		const GuildData& get_data() const;
		const GuildMember* find(int32_t cid) const;
		// The client takes the first member for the master.
		bool is_master(int32_t cid) const;
		// A rank's title, or "" for none.
		const std::string& title(int32_t rank) const;
		size_t online_count() const;
		// Members who would hear guild chat: those online but the player.
		std::vector<int32_t> online_ids(int32_t self) const;

		void update(GuildData data);
		void clear();

	private:
		GuildData data;
	};
}
