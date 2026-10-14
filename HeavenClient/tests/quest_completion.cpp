#include "../Character/QuestCompletion.h"

#include <cassert>

int main()
{
	using ms::QuestObjective;
	using Started = std::map<int16_t, std::vector<QuestObjective>>;

	auto snails = [](int32_t have) { return std::vector<QuestObjective>{ { true, 100100, have, 10 } }; };

	ms::QuestCompletion watch;

	// Seen for the first time: already met or not, no notice.
	assert(watch.update({ { 1037, snails(9) }, { 1008, snails(10) } }).empty());
	// The tenth snail.
	assert((watch.update({ { 1037, snails(10) }, { 1008, snails(10) } }) == std::vector<int16_t>{ 1037 }));
	// Still met: only once.
	assert(watch.update({ { 1037, snails(10) } }).empty());

	// Losing an item and finding another counts again.
	std::vector<QuestObjective> shell { { false, 4000019, 0, 1 } };
	assert(watch.update({ { 2000, shell } }).empty());
	shell[0].have = 1;
	assert((watch.update({ { 2000, shell } }) == std::vector<int16_t>{ 2000 }));
	shell[0].have = 0;
	assert(watch.update({ { 2000, shell } }).empty());
	shell[0].have = 1;
	assert((watch.update({ { 2000, shell } }) == std::vector<int16_t>{ 2000 }));

	// Nothing to hunt or bring: never complete here.
	assert(watch.update({ { 3000, {} } }).empty());
	assert(watch.update({ { 3000, {} } }).empty());

	// Started again after being forgotten: judged afresh.
	assert(watch.update({}).empty());
	assert(watch.update({ { 1037, snails(10) } }).empty());

	return 0;
}
