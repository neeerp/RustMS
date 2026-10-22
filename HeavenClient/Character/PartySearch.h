#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>

namespace ms
{
	// The Party Search a leader runs (reports b034, b036). v83 keeps it in
	// its context (ctx+0x3014..0x302c), not in the window, so it outlives
	// the window and a channel change. Pure state: each change says what to
	// send, and PartySearchActions sends it.
	class PartySearch
	{
	public:
		enum class State : uint8_t
		{
			IDLE,
			SEARCHING,
			PAUSED
		};

		enum class Send : uint8_t
		{
			NOTHING,
			// PARTY_SEARCH_START with the wanted search.
			START,
			// PARTY_SEARCH_STOP, for a pause as for a stop.
			STOP
		};

		struct Wanted
		{
			int32_t min_level = 0;
			int32_t max_level = 0;
			int32_t members = 0;
			int32_t jobs = 0;
		};

		// The window's 25 job boxes.
		static constexpr size_t BOXES = 25;
		static constexpr int32_t ALL_JOBS = (1 << BOXES) - 1;

		// What the window shows first (0x87989f): 15 levels either side,
		// six members, every job.
		static Wanted defaults(int16_t level);

		// Whether the window shows job box `box` to a player of `level`:
		// below 30, only All, Beginner, Aran and the class heads (0x878847).
		static bool box_shown(size_t box, int16_t level);
		// Its label: v83's job name for the job the box stands for at the
		// player's tier (0x4a77ef). All has none.
		static std::string box_label(size_t box, int16_t level);
		// Whether it is a class head, labelled darker.
		static bool box_heads(size_t box);
		// `jobs` after a click on `box`: All sets every box, a head its
		// class, and then each head and All show whether all theirs are set
		// (0x878ed3).
		static int32_t toggled(int32_t jobs, size_t box);

		// v83's checks before it searches (0x87943e), in its order: what
		// it says about the first one failed, or nothing if all pass. Mask
		// bits past the boxes fail silently, as "".
		static std::optional<std::string> refusal(const Wanted& wanted, int16_t level, size_t party_size);

		static PartySearch& get();

		State state() const;
		const Wanted& wanted() const;

		// Searching for `wanted`, already checked.
		Send start(const Wanted& wanted);
		// Searching again for what was asked before, unchecked, as the
		// indicator's click does (0xa12073).
		Send resume();
		Send pause();
		// Forgets the search.
		Send stop();

		// A channel change or the Cash Shop is coming (0x776020, 0x776a4f):
		// the next field resumes the search instead of ending it.
		void leaving_channel();
		// Every field entry (0xa03f2e).
		Send field_entered();
		// A member joined, making the party `size` (0xa3eb2d).
		Send joined(size_t size);

		// Logging out forgets it (0xa01dcf).
		void reset();

	private:
		State current = State::IDLE;
		Wanted saved;
		bool resuming = false;
	};
}
