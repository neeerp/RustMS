#include "../Character/PartySearch.h"

#include <iostream>
#include <stdexcept>

using namespace ms;

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition) throw std::runtime_error(message);
	}

	using Send = PartySearch::Send;
	using State = PartySearch::State;

	PartySearch::Wanted wanted(int32_t min, int32_t max, int32_t members, int32_t jobs = PartySearch::ALL_JOBS)
	{
		PartySearch::Wanted w;
		w.min_level = min;
		w.max_level = max;
		w.members = members;
		w.jobs = jobs;

		return w;
	}

	void defaults_surround_the_level()
	{
		PartySearch::Wanted low = PartySearch::defaults(8);
		require(low.min_level == 0 && low.max_level == 23, "low defaults");
		PartySearch::Wanted high = PartySearch::defaults(195);
		require(high.min_level == 180 && high.max_level == 200, "high defaults");
		require(high.members == 6 && high.jobs == PartySearch::ALL_JOBS, "members and jobs");
		require(!PartySearch::refusal(PartySearch::defaults(50), 50, 1), "defaults pass");
	}

	void checks_in_v83s_order()
	{
		auto refusal = [](PartySearch::Wanted w, int16_t level = 30, size_t size = 1)
		{
			return PartySearch::refusal(w, level, size).value_or("ok");
		};

		require(refusal(wanted(190, 201, 6)) == "Currently, the highest level in MapleStory\r\nis at 200!", "max");
		require(refusal(wanted(20, 51, 6)).find("30 levels") != std::string::npos, "span");
		require(refusal(wanted(40, 20, 6)) == "The min. value is higher than the max!", "backwards");
		require(refusal(wanted(31, 40, 6)).find("your own level") != std::string::npos, "own level");
		require(refusal(wanted(20, 40, 7)) == "You may enter with 2-6 party members.", "count");
		require(refusal(wanted(20, 40, 3), 30, 3) == "Your party already has 3 or more members.", "size");
		require(refusal(wanted(20, 40, 6, 0)).find("Job class") != std::string::npos, "no jobs");
		require(refusal(wanted(20, 40, 6, 1 << 25)).empty(), "unknown box, silently");
		require(refusal(wanted(INT32_MIN, 40, 6)).find("30 levels") != std::string::npos, "no overflow");
		require(refusal(wanted(20, 40, 6)) == "ok", "valid");
	}

	void pauses_and_resumes()
	{
		PartySearch search;
		require(search.pause() == Send::NOTHING && search.resume() == Send::NOTHING, "idle");
		require(search.start(wanted(20, 40, 4)) == Send::START, "start");
		require(search.start(wanted(20, 40, 5)) == Send::NOTHING, "already searching");
		require(search.pause() == Send::STOP && search.state() == State::PAUSED, "pause");
		require(search.resume() == Send::START && search.wanted().members == 4, "resume the saved search");
		require(search.stop() == Send::STOP && search.state() == State::IDLE, "stop");
		require(search.wanted().members == 0 && search.stop() == Send::NOTHING, "forgotten");
	}

	void a_new_field_ends_it_unless_the_channel_changed()
	{
		PartySearch search;
		require(search.field_entered() == Send::STOP, "every field entry tells the server");
		search.start(wanted(20, 40, 4));
		search.leaving_channel();
		require(search.field_entered() == Send::START && search.state() == State::SEARCHING, "resumed");
		require(search.field_entered() == Send::STOP && search.state() == State::IDLE, "a map change ends it");

		search.start(wanted(20, 40, 4));
		search.pause();
		search.leaving_channel();
		require(search.field_entered() == Send::NOTHING && search.state() == State::PAUSED, "still paused");
	}

	void a_full_party_ends_it()
	{
		PartySearch search;
		require(search.joined(2) == Send::NOTHING, "nothing to end");
		search.start(wanted(20, 40, 3));
		require(search.joined(2) == Send::NOTHING, "not yet");
		require(search.joined(3) == Send::STOP && search.state() == State::IDLE, "big enough");
	}

	void boxes_follow_their_class()
	{
		int32_t all = PartySearch::ALL_JOBS;
		int32_t fighter = 1 << 4;
		int32_t warriors = 0xf8;

		require(PartySearch::toggled(all, 0) == 0, "All clears every box");
		require(PartySearch::toggled(0, 0) == all, "and sets them");
		require(PartySearch::toggled(0, 3) == warriors, "a head sets its class");
		require(PartySearch::toggled(warriors, 4) == (warriors & ~fighter & ~(1 << 3)), "a branch unsets its head");
		require(PartySearch::toggled(all, 4) == (all & ~fighter & ~(1 << 3) & ~1), "and All");
		require(PartySearch::toggled(warriors & ~fighter & ~(1 << 3), 4) == warriors, "the last branch sets its head");
		require(PartySearch::toggled(all & ~2, 1) == all, "the last box sets All");
		require(PartySearch::toggled(0, 1) == 2, "Beginner stands alone");
		require(PartySearch::toggled(0, 24) == 1 << 24, "one archer branch leaves the head unset");
	}

	void boxes_show_the_players_tier()
	{
		require(PartySearch::box_label(0, 50).empty(), "All has no label");
		require(PartySearch::box_label(4, 69) == "Fighter", "first job");
		require(PartySearch::box_label(4, 70) == "Crusader", "from 70");
		require(PartySearch::box_label(4, 120) == "Hero", "from 120");
		require(PartySearch::box_label(9, 100) == "Mage(Fire, Poison)", "the pool's spacing");
		require(PartySearch::box_shown(21, 29) && !PartySearch::box_shown(22, 29), "below 30, heads only");
		require(PartySearch::box_shown(2, 10) && PartySearch::box_shown(22, 30), "Aran, and all from 30");
		require(PartySearch::box_heads(17) && !PartySearch::box_heads(1), "heads");
	}
}

int main()
{
	try
	{
		defaults_surround_the_level();
		checks_in_v83s_order();
		pauses_and_resumes();
		a_new_field_ends_it_unless_the_channel_changed();
		a_full_party_ends_it();
		boxes_follow_their_class();
		boxes_show_the_players_tier();
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
