#include "PartySearch.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		constexpr int32_t MAX_LEVEL = 200;
		constexpr int32_t LEVEL_SPAN = 30;
		constexpr int32_t AROUND = 15;
		constexpr int32_t MOST_MEMBERS = 6;

		// The class heads, each followed by its branches up to the next.
		struct Group
		{
			size_t head, last;
		};

		constexpr Group GROUPS[] = { { 3, 7 }, { 8, 12 }, { 13, 16 }, { 17, 20 }, { 21, 24 } };

		// Each box's labels for the tiers below 70, to 119, and from 120.
		// The pool's own spacing, "Mage(Fire, Poison)" and all.
		const char* const LABELS[PartySearch::BOXES][3] = {
			{ "", "", "" },
			{ "Beginner", "Beginner", "Beginner" },
			{ "Aran", "Aran", "Aran" },
			{ "Swordman", "Swordman", "Swordman" },
			{ "Fighter", "Crusader", "Hero" },
			{ "Page", "White Knight", "Paladin" },
			{ "Spearman", "Dragon Knight", "Dark Knight" },
			{ "Dawn Warrior", "Dawn Warrior", "Dawn Warrior" },
			{ "Magician", "Magician", "Magician" },
			{ "Wizard (Fire,Poison)", "Mage(Fire, Poison)", "Arch Mage(Fire,Poison)" },
			{ "Wizard(Ice,Lightning)", "Mage(Ice,Lightning)", "Arch Mage(Ice,Lightning)" },
			{ "Cleric", "Priest", "Bishop" },
			{ "Blaze Wizard", "Blaze Wizard", "Blaze Wizard" },
			{ "Pirate", "Pirate", "Pirate" },
			{ "Brawler", "Marauder", "Buccaneer" },
			{ "Gunslinger", "Outlaw", "Corsair" },
			{ "Thunder Breaker", "Thunder Breaker", "Thunder Breaker" },
			{ "Rogue", "Rogue", "Rogue" },
			{ "Assassin", "Hermit", "Night Lord" },
			{ "Bandit", "Chief Bandit", "Shadower" },
			{ "Night Walker", "Night Walker", "Night Walker" },
			{ "Archer", "Archer", "Archer" },
			{ "Hunter", "Ranger", "Bowmaster" },
			{ "Crossbow man", "Sniper", "Marksman" },
			{ "Wind Archer", "Wind Archer", "Wind Archer" }
		};

		// Boxes `first` to `last`.
		int32_t bits(size_t first, size_t last)
		{
			return ((1 << (last + 1)) - 1) & ~((1 << first) - 1);
		}
	}

	PartySearch::Wanted PartySearch::defaults(int16_t level)
	{
		Wanted wanted;
		wanted.min_level = std::max<int32_t>(level - AROUND, 0);
		wanted.max_level = std::min<int32_t>(level + AROUND, MAX_LEVEL);
		wanted.members = MOST_MEMBERS;
		wanted.jobs = ALL_JOBS;

		return wanted;
	}

	bool PartySearch::box_shown(size_t box, int16_t level)
	{
		return level >= 30 || box <= 2 || box_heads(box);
	}

	std::string PartySearch::box_label(size_t box, int16_t level)
	{
		if (box >= BOXES)
			return "";

		size_t tier = level < 70 ? 0 : level < 120 ? 1 : 2;

		return LABELS[box][tier];
	}

	bool PartySearch::box_heads(size_t box)
	{
		return std::any_of(std::begin(GROUPS), std::end(GROUPS), [box](const Group& group)
		{
			return group.head == box;
		});
	}

	int32_t PartySearch::toggled(int32_t jobs, size_t box)
	{
		if (box >= BOXES)
			return jobs;

		int32_t changed = 1 << box;

		if (box == 0)
			changed = ALL_JOBS;

		for (const Group& group : GROUPS)
			if (group.head == box)
				changed = bits(group.head, group.last);

		jobs = jobs & (1 << box) ? jobs & ~changed : jobs | changed;

		for (const Group& group : GROUPS)
		{
			int32_t branches = bits(group.head + 1, group.last);
			int32_t head = 1 << group.head;

			jobs = (jobs & branches) == branches ? jobs | head : jobs & ~head;
		}

		int32_t others = ALL_JOBS & ~1;

		return (jobs & others) == others ? jobs | 1 : jobs & ~1;
	}

	std::optional<std::string> PartySearch::refusal(const Wanted& wanted, int16_t level, size_t party_size)
	{
		if (wanted.max_level > MAX_LEVEL)
			return "Currently, the highest level in MapleStory\r\nis at 200!";

		if (static_cast<int64_t>(wanted.max_level) - wanted.min_level > LEVEL_SPAN)
			return "You can only search for party\r\nmembers within a range of 30 levels.";

		if (wanted.min_level > wanted.max_level)
			return "The min. value is higher than the max!";

		if (level < wanted.min_level || level > wanted.max_level)
			return "The range of level for search has to include your own level.";

		if (wanted.members < 2 || wanted.members > MOST_MEMBERS)
			return "You may enter with 2-6 party members.";

		if (static_cast<size_t>(wanted.members) <= party_size)
			return "Your party already has " + std::to_string(wanted.members) + " or more members.";

		if (wanted.jobs == 0)
			return "Please enter the Job class you'd like to party with.";

		if (wanted.jobs & ~ALL_JOBS)
			return "";

		return std::nullopt;
	}

	PartySearch& PartySearch::get()
	{
		static PartySearch search;

		return search;
	}

	PartySearch::State PartySearch::state() const
	{
		return current;
	}

	const PartySearch::Wanted& PartySearch::wanted() const
	{
		return saved;
	}

	PartySearch::Send PartySearch::start(const Wanted& wanted)
	{
		if (current == State::SEARCHING)
			return Send::NOTHING;

		saved = wanted;
		current = State::SEARCHING;

		return Send::START;
	}

	PartySearch::Send PartySearch::resume()
	{
		if (current != State::PAUSED)
			return Send::NOTHING;

		current = State::SEARCHING;

		return Send::START;
	}

	PartySearch::Send PartySearch::pause()
	{
		if (current != State::SEARCHING)
			return Send::NOTHING;

		current = State::PAUSED;

		return Send::STOP;
	}

	PartySearch::Send PartySearch::stop()
	{
		if (current == State::IDLE)
			return Send::NOTHING;

		current = State::IDLE;
		saved = Wanted();

		return Send::STOP;
	}

	void PartySearch::leaving_channel()
	{
		resuming = true;
	}

	PartySearch::Send PartySearch::field_entered()
	{
		if (resuming)
		{
			resuming = false;

			return current == State::SEARCHING ? Send::START : Send::NOTHING;
		}

		// HC's earlier reading of 0xa03eba: 0xDF goes out on every other
		// field entry, searching or not.
		stop();

		return Send::STOP;
	}

	PartySearch::Send PartySearch::joined(size_t size)
	{
		if (size < static_cast<size_t>(std::max(saved.members, 0)))
			return Send::NOTHING;

		return stop();
	}

	void PartySearch::reset()
	{
		*this = PartySearch();
	}
}
